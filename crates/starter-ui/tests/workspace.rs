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

#[test]
fn optional_map_panel_migrates_and_roundtrips_without_duplicates() {
    let desktop = Workspace::default();
    assert!(!desktop.is_visible(PanelId::Map));
    let mut bevy: Workspace =
        serde_json::from_str(&serde_json::to_string(&desktop).unwrap()).unwrap();
    bevy.ensure_panel(PanelId::Map);
    bevy.activate(PanelId::Map);
    assert!(bevy.is_visible(PanelId::Map));
    let encoded = serde_json::to_string(&bevy).unwrap();
    bevy.ensure_panel(PanelId::Map);
    assert_eq!(encoded, serde_json::to_string(&bevy).unwrap());
    let mut restored: Workspace = serde_json::from_str(&encoded).unwrap();
    restored.set_visible(PanelId::Map, false);
    restored.activate(PanelId::Viewport);
    assert!(restored.is_visible(PanelId::Viewport));
}

#[test]
fn dragging_splitter_resizes_panes() {
    use egui::{Rect, pos2, vec2};
    use egui_kittest::Harness;
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut (Workspace, Rect, Rect)| {
            let (workspace, explorer, viewport) = state;
            workspace.show(ui, |ui, pane| {
                if pane == PanelId::Explorer {
                    *explorer = ui.max_rect();
                }
                if pane == PanelId::Viewport {
                    *viewport = ui.max_rect();
                }
                ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
            });
        },
        (Workspace::default(), Rect::NOTHING, Rect::NOTHING),
    );
    harness.run();
    let before = harness.state().1;
    let viewport = harness.state().2;
    let splitter = pos2((before.right() + viewport.left()) * 0.5, before.center().y);
    harness.hover_at(splitter);
    harness.run();
    harness.drag_at(splitter);
    harness.run_steps(2);
    harness.hover_at(splitter + vec2(80.0, 0.0));
    harness.run_steps(2);
    harness.drop_at(splitter + vec2(80.0, 0.0));
    harness.run();
    assert!(harness.state().1.width() > before.width() + 50.0);
}
