use super::Node;
use crate::project::{Project, ProjectIo};
use starter_core::{LayerKind, ProjectSelection};
use starter_ui::TreeViewBuilder;

pub fn build(b: &mut TreeViewBuilder<'_, Node>, project: &Project, io: &ProjectIo) {
    if b.dir(Node::Project, &project.document.name) {
        b.leaf(Node::Manifest, "project.json");
        if b.dir(Node::Assets, "Assets") {
            for (node, name, prefix) in [
                (Node::Models, "Models", "Assets/Models/"),
                (Node::Textures, "Textures", "Assets/Textures/"),
            ] {
                if b.dir(node, name) {
                    for (i, asset) in project
                        .document
                        .assets
                        .iter()
                        .enumerate()
                        .filter(|(_, a)| a.path.starts_with(prefix))
                    {
                        let name = asset.path.strip_prefix(prefix).unwrap_or(&asset.path);
                        b.leaf(
                            Node::File(i),
                            format!(
                                "{name}{}",
                                if io.missing.get(i) == Some(&true) {
                                    " [missing]"
                                } else {
                                    ""
                                }
                            ),
                        );
                    }
                }
                b.close_dir();
            }
        }
        b.close_dir();
        if b.dir(Node::Geodata, "Geodata") {
            for kind in [LayerKind::Terrain, LayerKind::Satellite] {
                let layer = project.document.layer(kind);
                b.leaf(
                    Node::Layer(kind),
                    format!(
                        "{kind:?} · {}",
                        layer.package_id.as_deref().unwrap_or("not loaded")
                    ),
                );
            }
        }
        b.close_dir();
    }
    b.close_dir();
}
pub fn show_manager(project: &Project, io: &mut ProjectIo) {
    let Some(path) = io.path.clone() else {
        io.error = Some("Save the project first".into());
        return;
    };
    let asset = if let ProjectSelection::Asset(i) = project.selection {
        project.document.assets.get(i).map(|a| a.path.clone())
    } else {
        None
    };
    bevy::tasks::AsyncComputeTaskPool::get()
        .spawn(async move {
            let Some(root) = path.parent() else {
                return;
            };
            let directory = asset
                .as_ref()
                .and_then(|a| crate::project::storage::asset_path(root, a).ok())
                .and_then(|a| a.parent().map(std::path::Path::to_owned))
                .unwrap_or_else(|| root.to_owned());
            let program = match std::env::consts::OS {
                "windows" => "explorer",
                "macos" => "open",
                _ => "xdg-open",
            };
            let _ = std::process::Command::new(program).arg(directory).spawn();
        })
        .detach();
}
