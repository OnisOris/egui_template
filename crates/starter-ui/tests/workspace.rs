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
