//! UAV v2 manifest compatibility; only project-owned pixels, no external HTTP.
use super::{package_browser, satellite};
#[test]
fn actual_uav_v2_manifest_ignores_vectors_and_uses_the_existing_decoder() {
    let manifest: satellite::MapManifest =
        serde_json::from_str(include_str!("../../tests/fixtures/uav-map-data-v2.json")).unwrap();
    manifest.validate(None).unwrap();
    assert_ne!(manifest.origin.longitude, manifest.bbox.center().longitude);
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("package");
    std::fs::create_dir(&dir).unwrap();
    let json = include_bytes!("../../tests/fixtures/uav-map-data-v2.json");
    std::fs::write(dir.join("manifest.json"), json).unwrap();
    let m = &manifest.satellite;
    image::RgbImage::from_fn(m.width, m.height, |x, y| {
        image::Rgb([
            if x < m.width / 2 { 230 } else { 30 },
            if y < m.height / 2 { 200 } else { 20 },
            80,
        ])
    })
    .save(dir.join("satellite.jpg"))
    .unwrap();
    let first = satellite::read_package(&dir, None).unwrap();
    let second = satellite::read_package(&dir, None).unwrap();
    assert_eq!(first.fingerprint, second.fingerprint);
    assert!(first.uvs.is_empty());
    assert_eq!(first.rgba.len(), m.width as usize * m.height as usize * 4);
    assert!(first.rgba[1] > first.rgba[first.rgba.len() - 3]);
    let found = package_browser::scan(root.path()).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1.map_data_id, manifest.map_data_id);
    assert_eq!(std::fs::read(dir.join("manifest.json")).unwrap(), json);
    std::fs::write(dir.join("satellite.jpg"), b"broken").unwrap();
    assert!(package_browser::scan(root.path()).unwrap().is_empty());
}
