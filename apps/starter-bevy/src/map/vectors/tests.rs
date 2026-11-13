use super::{clip, data::*, mesh, package};
use crate::map::{dem_tests, satellite_tests, source};
use bevy::mesh::VertexAttributeValues;
fn ring(x: f64, y: f64, size: f64) -> Vec<LocalPoint2> {
    [
        (x, y),
        (x + size, y),
        (x + size, y + size),
        (x, y + size),
        (x, y),
    ]
    .into_iter()
    .map(|(x, y)| LocalPoint2 { x, y })
    .collect()
}
#[test]
fn bincode_package_validation_holes_multipolygon_and_terrain_draping() {
    let dem = dem_tests::package(6, 6);
    let terrain = source::load_terrain(dem.path()).unwrap();
    let dir = satellite_tests::package(&terrain);
    let roads = vec![RoadFeature {
        id: "road".into(),
        class: RoadClass::Minor,
        points: vec![
            LocalPoint2 { x: 1., y: 10. },
            LocalPoint2 { x: 49., y: 10. },
        ],
    }];
    let buildings = vec![BuildingFeature {
        id: "house".into(),
        height_m: 12.,
        min_height_m: 1.,
        polygons: vec![
            BuildingPolygon {
                rings: vec![ring(2., 2., 20.), ring(6., 6., 4.)],
            },
            BuildingPolygon {
                rings: vec![ring(30., 30., 5.)],
            },
        ],
    }];
    for (name, bytes) in [
        (
            "roads.bin",
            bincode::serde::encode_to_vec(&roads, bincode::config::standard()).unwrap(),
        ),
        (
            "buildings.bin",
            bincode::serde::encode_to_vec(&buildings, bincode::config::standard()).unwrap(),
        ),
    ] {
        std::fs::write(dir.path().join(name), bytes).unwrap();
    }
    let path = dir.path().join("manifest.json");
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    json["vectors"] = serde_json::json!({"provider":"fixture","zoom":14,"tileCount":1,"roadCount":1,"buildingCount":1,
        "roadsFile":"roads.bin","buildingsFile":"buildings.bin","binaryFormat":"bincode-v2-serde","attribution":"local fixture"});
    std::fs::write(&path, serde_json::to_vec(&json).unwrap()).unwrap();
    let data = package::load(dir.path(), terrain.manifest.region()).unwrap();
    assert_eq!(data.buildings[0].polygons.len(), 2);
    let flat = mesh::build(&data, &terrain, true);
    let combined = mesh::build(&data, &terrain, false);
    assert!(flat.len() <= 4 && combined.len() <= 4);
    let roof = flat.iter().find(|c| c.buildings).unwrap();
    let Some(VertexAttributeValues::Float32x3(p)) =
        roof.mesh.attribute(bevy::prelude::Mesh::ATTRIBUTE_POSITION)
    else {
        panic!()
    };
    let mut area = 0.;
    for t in p.chunks_exact(3).filter(|t| t.iter().all(|p| p[1] == 12.)) {
        area += ((t[1][0] - t[0][0]) * (t[2][2] - t[0][2])
            - (t[2][0] - t[0][0]) * (t[1][2] - t[0][2]))
            .abs()
            / 2.;
    }
    assert!(
        (area - (400. - 16. + 25.)).abs() < 0.01,
        "hole was filled: {area}"
    );
    let road = combined.iter().find(|c| !c.buildings).unwrap();
    let Some(VertexAttributeValues::Float32x3(p)) =
        road.mesh.attribute(bevy::prelude::Mesh::ATTRIBUTE_POSITION)
    else {
        panic!()
    };
    assert!(p.iter().any(|p| p[1] > 5.));
    assert_eq!(terrain.position(5, 5)[2], 15.);
    let mut wrong = terrain.manifest.region();
    wrong.origin.latitude += 0.001;
    assert!(package::load(dir.path(), wrong).is_err());
    std::fs::write(dir.path().join("roads.bin"), [255u8; 8]).unwrap();
    assert!(package::load(dir.path(), terrain.manifest.region()).is_err());
    assert!(package::decode::<Vec<RoadFeature>>(&[0, 1]).is_err());
}
#[test]
fn clipping_retains_intersections_and_rejects_invalid_geometry() {
    let dem = dem_tests::package(6, 6);
    let t = source::load_terrain(dem.path()).unwrap();
    let rect = clip::rectangle(t.manifest.region());
    let line = clip::roads(
        &[
            LocalPoint2 { x: -20., y: 10. },
            LocalPoint2 { x: 80., y: 10. },
        ],
        rect,
    );
    assert_eq!(line.len(), 1);
    assert!(line[0][0].x.abs() < 1e-8);
    assert!((line[0][1].x - 50.).abs() < 1e-6);
    let polygons = clip::buildings(
        &BuildingPolygon {
            rings: vec![ring(-10., -10., 30.)],
        },
        rect,
    );
    assert_eq!(polygons.len(), 1);
    assert!(polygons[0].rings[0].iter().all(|p| p.x >= 0. && p.y >= 0.));
}
