//! Explicit real-data/performance probe; never run by CI tests.
use starter_bevy::map::{
    builder::{Job, cache},
    source::load_terrain,
    vectors::{mesh, package, source},
};
use std::{
    path::Path,
    sync::{Arc, Mutex, atomic::AtomicBool},
    time::Instant,
};
fn main() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    let terrain = load_terrain(Path::new(
        args.get(1)
            .ok_or("vector_probe TERRAIN_DIR [VECTOR_PACKAGE_DIR]")?,
    ))?;
    let started = Instant::now();
    let data = if let Some(path) = args.get(2) {
        package::load(Path::new(path), terrain.manifest.region())?
    } else {
        source::load(
            terrain.manifest.region(),
            &cache::root()?,
            source::TILEJSON,
            &Job {
                cancelled: Arc::new(AtomicBool::new(false)),
                progress: Arc::new(Mutex::new(String::new())),
            },
        )?
    };
    println!(
        "{} buildings, {} roads, source={} load_ms={:.3} fingerprint={}",
        data.buildings.len(),
        data.roads.len(),
        data.source,
        started.elapsed().as_secs_f64() * 1000.,
        data.fingerprint
    );
    for flat in [true, false] {
        let started = Instant::now();
        let chunks = mesh::build(&data, &terrain, flat);
        let vertices: usize = chunks.iter().map(|c| c.mesh.count_vertices()).sum();
        println!(
            "flat={flat} chunks={} vertices={vertices} mesh_ms={:.3} materials=2",
            chunks.len(),
            started.elapsed().as_secs_f64() * 1000.
        );
    }
    Ok(())
}
