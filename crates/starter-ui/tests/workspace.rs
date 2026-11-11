use starter_ui::{PanelId, Workspace};

#[test]
fn dock_layout_persists_and_panels_can_be_hidden_and_restored() {
    let mut workspace = Workspace::default();
    workspace.set_visible(PanelId::Inspector, false);
    assert!(!workspace.is_visible(PanelId::Inspector));
    workspace.set_visible(PanelId::Inspector, true);

    let encoded = serde_json::to_string(&workspace).unwrap();
    let restored: Workspace = serde_json::from_str(&encoded).unwrap();
    assert_eq!(restored, workspace);
    assert!(restored.is_visible(PanelId::Inspector));
}

#[test]
fn closing_every_tab_keeps_the_dock_tree_stable() {
    use egui_kittest::{Harness, kittest::Queryable};

    let mut harness = Harness::new_ui_state(
        |ui, workspace: &mut Workspace| {
            workspace.show(ui, |ui, pane| {
                ui.heading(pane.title());
            });
        },
        Workspace::default(),
    );
    harness.run();

    for expected in (0..4).rev() {
        let closes: Vec<_> = harness.query_all_by_label("Close").collect();
        assert_eq!(closes.len(), expected + 1);
        closes[0].click();
        harness.run();
    }

    assert!(!harness.state().is_visible(PanelId::Viewport));
    assert!(!harness.state().is_visible(PanelId::DataTable));
    assert!(!harness.state().is_visible(PanelId::Charts));
    assert!(!harness.state().is_visible(PanelId::Components));

    harness.state_mut().set_visible(PanelId::Viewport, true);
    harness.run();
    assert!(harness.state().is_visible(PanelId::Viewport));
}
