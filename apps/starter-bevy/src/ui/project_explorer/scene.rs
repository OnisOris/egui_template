use super::Node;
use crate::{
    map::{loading::TerrainImport, real_scene::RealScene},
    project::Project,
};
use starter_core::LayerKind;
use starter_ui::TreeViewBuilder;
pub fn build(
    b: &mut TreeViewBuilder<'_, Node>,
    project: &Project,
    real: &RealScene,
    import: &TerrainImport,
) {
    if b.dir(Node::Project, "Scene") {
        if b.dir(Node::Map, "Map") {
            for kind in [LayerKind::Terrain, LayerKind::Satellite] {
                let layer = project.document.layer(kind);
                let ready = match kind {
                    LayerKind::Terrain => real.data.is_some(),
                    LayerKind::Satellite => real.satellite.is_some(),
                };
                let status = if ready {
                    "Ready"
                } else if import.builder.loading || import.loading() {
                    "Loading"
                } else if layer.package_id.is_some() {
                    "Missing cache · Load in 3D"
                } else {
                    "Not loaded"
                };
                b.leaf(
                    Node::Layer(kind),
                    format!(
                        "{} {kind:?} · {status}",
                        if layer.visible { "●" } else { "○" }
                    ),
                );
            }
        }
        b.close_dir();
        if b.dir(Node::Objects, "Objects") {
            for marker in &project.document.objects {
                b.leaf(
                    Node::Marker(marker.id),
                    format!(
                        "{}{}",
                        marker.name,
                        if marker.visible { "" } else { " [hidden]" }
                    ),
                );
            }
        }
        b.close_dir();
        b.dir(Node::Groups, "Groups");
        b.close_dir();
    }
    b.close_dir();
}
