use super::Node;
use crate::{
    map::{loading::TerrainImport, real_scene::RealScene},
    project::Project,
};
use starter_core::LayerKind;
use starter_ui::{NodeBuilder, TreeViewBuilder};
pub fn build(
    b: &mut TreeViewBuilder<'_, Node>,
    project: &Project,
    real: &RealScene,
    import: &TerrainImport,
) {
    if b.dir(Node::Project, "Scene") {
        if b.dir(Node::Map, "Map") {
            for kind in LayerKind::ALL {
                let layer = project.document.layer(kind);
                let current = project
                    .document
                    .map_region
                    .is_some_and(|r| real.data.as_ref().is_some_and(|d| d.manifest.matches(r)));
                let present = match kind {
                    LayerKind::Terrain => real.data.is_some(),
                    LayerKind::Satellite => real.satellite.is_some(),
                    LayerKind::Roads | LayerKind::Buildings => {
                        real.vector_status.starts_with("Vectors: Ready")
                    }
                };
                let ready = current && present;
                let vector = matches!(kind, LayerKind::Roads | LayerKind::Buildings);
                let status = if vector && real.vector_status.contains("Failed") {
                    "Failed"
                } else if vector
                    && (real.vector_status.contains("Loading")
                        || real.vector_status.contains("Reading TileJSON")
                        || real.vector_status.contains("geometry"))
                {
                    "Loading"
                } else if ready {
                    "Ready"
                } else if import.builder.loading || import.loading() {
                    "Loading"
                } else if present && !current {
                    "Previous region"
                } else if layer.package_id.is_some() {
                    "Missing cache · Load in 3D"
                } else {
                    "Not loaded"
                };
                b.node(
                    NodeBuilder::leaf(Node::Layer(kind))
                        .icon(move |ui| {
                            ui.label(match kind {
                                LayerKind::Terrain => "▧",
                                LayerKind::Satellite => "▤",
                                LayerKind::Roads => "≋",
                                LayerKind::Buildings => "⌂",
                            });
                        })
                        .label(format!(
                            "{} {kind:?} · {status}",
                            if layer.visible { "●" } else { "○" }
                        )),
                );
            }
        }
        b.close_dir();
        if b.dir(Node::Objects, "Objects") {
            for (node, label, models) in [
                (Node::MapMarkers, "Map Markers", false),
                (Node::ModelInstances, "Model Instances", true),
            ] {
                if b.dir(node, label) {
                    for marker in project
                        .document
                        .object_rows()
                        .filter(|m| (m.kind == "Model") == models)
                    {
                        b.leaf(
                            if models {
                                Node::Model(marker.id)
                            } else {
                                Node::Marker(marker.id)
                            },
                            format!(
                                "{}{}",
                                marker.name,
                                if marker.visible { "" } else { " [hidden]" }
                            ),
                        );
                    }
                }
                b.close_dir();
            }
        }
        b.close_dir();
    }
    b.close_dir();
}
