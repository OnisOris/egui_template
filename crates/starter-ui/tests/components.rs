use egui_kittest::{Harness, kittest::Queryable};
use starter_ui::Workspace;

#[test]
fn docking_workspace_renders_default_panels() {
    let mut workspace = Workspace::default();
    let mut harness = Harness::new_ui(|ui| {
        workspace.show(ui, |ui, pane| {
            ui.heading(pane.title());
        });
    });
    harness.run_ok();
    assert!(harness.query_all_by_label("Explorer").next().is_some());
    assert!(harness.query_all_by_label("Viewport").next().is_some());
    assert!(harness.query_all_by_label("Inspector").next().is_some());
}
