//! Synthetic CPU scale check; this is not native GUI or a claim about OSM coverage.
use super::{data::*, mesh};
#[test]
fn bounded_batches_for_thirty_two_one_thousand_and_ten_thousand_buildings() {
    let dir = crate::map::dem_tests::package(201, 201);
    let terrain = crate::map::source::load_terrain(dir.path()).unwrap();
    for count in [32, 1000, 10_000] {
        let mut data = super::mvt::empty(terrain.manifest.region(), "authored-load-fixture".into());
        for i in 0..count {
            let x = f64::from(i % 100) * 20. + 1.;
            let y = f64::from(i / 100) * 20. + 1.;
            let rings = vec![
                [
                    (x, y),
                    (x + 10., y),
                    (x + 10., y + 10.),
                    (x, y + 10.),
                    (x, y),
                ]
                .into_iter()
                .map(|(x, y)| LocalPoint2 { x, y })
                .collect(),
            ];
            data.buildings.push(BuildingFeature {
                id: i.to_string(),
                polygons: vec![BuildingPolygon { rings }],
                height_m: 12.,
                min_height_m: 0.,
            });
        }
        super::clip::validate(&data).unwrap();
        for flat in [true, false] {
            let started = std::time::Instant::now();
            let chunks = mesh::build(&data, &terrain, flat);
            let vertices: usize = chunks.iter().map(|c| c.mesh.count_vertices()).sum();
            println!(
                "authored buildings={count} flat={flat} chunks={} vertices={vertices} CPU_mesh_ms={:.3}",
                chunks.len(),
                started.elapsed().as_secs_f64() * 1000.
            );
            assert!(chunks.len() <= 16);
            assert_eq!(vertices, count as usize * 30);
        }
    }
}
