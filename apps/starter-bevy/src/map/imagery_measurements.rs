//! Repeatable CPU/local-network measurements, not a public-provider or GPU benchmark.
use super::{
    builder::coverage,
    coordinates::local_to_wgs84,
    dem_tests, imagery_3d,
    imagery_fixture::{Fixture, job},
    imagery_service::TileService,
    region::{GeoBounds, MapRegion},
    source,
};
use std::time::Instant;
#[test]
fn bounded_one_five_and_fifteen_km_regions_reuse_all_warm_tiles() {
    let fixture = Fixture::new();
    let dir = dem_tests::package(3, 3);
    let terrain = source::load_terrain(dir.path()).unwrap();
    for side in [1000.0, 5000.0, 15000.0] {
        let service = TileService::new(fixture.provider.clone(), None).unwrap();
        let origin = terrain.manifest.origin;
        let (east, south) = local_to_wgs84(origin, side, side);
        let region = MapRegion {
            origin,
            bounds: GeoBounds {
                west: origin.longitude,
                north: origin.latitude,
                east,
                south,
            },
        };
        let desired = coverage::zoom(region, 0, 19, 256, 8.0);
        let start = Instant::now();
        let image =
            imagery_3d::prepare(&service, &terrain, region, desired, &job(), false).unwrap();
        let cold_ms = start.elapsed().as_secs_f64() * 1000.0;
        let cold = service.stats();
        let start = Instant::now();
        let warm = imagery_3d::prepare(&service, &terrain, region, desired, &job(), false).unwrap();
        let warm_ms = start.elapsed().as_secs_f64() * 1000.0;
        let stats = service.stats();
        assert_eq!(
            stats.0, cold.0,
            "warm coverage must make no extra HTTP requests"
        );
        assert_eq!(image.rgba, warm.rgba);
        assert!(image.metadata.width <= 4096 && image.metadata.height <= 4096);
        assert!(stats.2 <= 128 * 1024 * 1024);
        println!(
            "LOCAL_FIXTURE side_km={} z={} image={}x{} http={} reused_tiles={} ready_cache_lookups={} decoded_bytes={} decode_ms={:.2} prepare_ms={:.2} cold_ms={:.2} warm_ms={:.2}",
            side / 1000.0,
            image.metadata.zoom,
            image.metadata.width,
            image.metadata.height,
            stats.0,
            cold.0,
            stats.1 - cold.1,
            stats.2,
            stats.3,
            image.preparation_ms,
            cold_ms,
            warm_ms
        );
    }
}
