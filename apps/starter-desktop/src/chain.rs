//! End-to-end flow: selection -> inspector edit -> document -> table -> viewport picking.

use eframe::egui::{self, Key, Modifiers, PointerButton, Pos2, accesskit::Role};
use egui_kittest::{Harness, kittest::Queryable};
use starter_core::{Command, DocumentState, ObjectId, Position};
use starter_ui::ExplorerTree;

use crate::{
    commands,
    features::{
        explorer::{self, ExplorerNodeId},
        inspector::{self, InspectorState},
        scene::{self, ViewportState},
        table::{self, TableState},
    },
};

struct Chain {
    state: DocumentState,
    notice: Option<String>,
    inspector: InspectorState,
    table: TableState,
    viewport: ViewportState,
    explorer: ExplorerTree<ExplorerNodeId>,
    pending: Vec<Command>,
}

impl Chain {
    fn new() -> Self {
        Self {
            state: DocumentState::default(),
            notice: None,
            inspector: InspectorState::default(),
            table: TableState::default(),
            viewport: ViewportState::default(),
            explorer: ExplorerTree::default(),
            pending: Vec::new(),
        }
    }

    fn dispatch(&mut self, command: Command) {
        commands::dispatch(&mut self.state, &mut self.notice, command);
    }
}

fn flush(harness: &mut Harness<Chain>) {
    let chain = harness.state_mut();
    let pending = std::mem::take(&mut chain.pending);
    for command in pending {
        chain.dispatch(command);
    }
}

fn click(harness: &mut Harness<Chain>, pos: Pos2) {
    harness.hover_at(pos);
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::default(),
        });
    }
}

fn type_name(harness: &mut Harness<Chain>, text: &str) {
    harness.get_by_role(Role::TextInput).click();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.get_by_role(Role::TextInput).type_text(text);
    harness.key_press(Key::Enter);
    harness.run();
    flush(harness);
    harness.run();
}

fn inspector_harness(chain: Chain) -> Harness<'static, Chain> {
    let mut harness = Harness::builder()
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, chain: &mut Chain| {
                let pending = inspector::show(ui, &chain.state, &mut chain.inspector);
                chain.pending.extend(pending);
            },
            chain,
        );
    harness.run();
    harness
}

fn viewport_harness(chain: Chain) -> Harness<'static, Chain> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 480.0))
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, chain: &mut Chain| {
                let pending = scene::show(ui, &chain.state, &mut chain.viewport);
                chain.pending.extend(pending);
            },
            chain,
        );
    harness.run();
    harness
}

fn explorer_harness(chain: Chain) -> Harness<'static, Chain> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(240.0, 320.0))
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, chain: &mut Chain| {
                let pending = explorer::show(ui, &chain.state, &mut chain.explorer);
                chain.pending.extend(pending);
            },
            chain,
        );
    harness.run();
    harness
}

#[test]
fn invalid_name_keeps_the_draft_editable() {
    let mut chain = Chain::new();
    chain.dispatch(Command::Select([ObjectId(1)].into()));
    let mut harness = inspector_harness(chain);

    type_name(&mut harness, "   ");

    let chain = harness.state();
    assert!(chain.notice.is_some());
    assert_eq!(chain.state.object(ObjectId(1)).unwrap().name(), "Object 1");
    assert_eq!(chain.inspector.name_draft.as_ref().unwrap().1, "   ");

    type_name(&mut harness, "Renamed");

    let chain = harness.state();
    assert!(chain.notice.is_none());
    assert_eq!(chain.state.object(ObjectId(1)).unwrap().name(), "Renamed");
}

#[test]
fn viewport_selection_reaches_inspector_and_table() {
    let mut chain = Chain::new();
    let target = ObjectId(1);
    for command in [
        Command::SetPosition(target, Position { x: 0.0, y: 0.0 }),
        Command::SetSize(target, 50.0),
    ] {
        chain.dispatch(command);
    }

    let mut harness = viewport_harness(chain);
    let center = egui::pos2(320.0, 240.0);
    click(&mut harness, center);
    harness.run();
    flush(&mut harness);
    harness.run();

    let chain = harness.state();
    assert_eq!(chain.notice, None);
    assert!(chain.state.selection().contains(&target));

    let mut harness = inspector_harness(harness.into_state());
    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some("Object 1")
    );
    type_name(&mut harness, "Renamed");

    let chain = harness.into_state();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 600.0))
        .with_accessibility_check(false)
        .build_ui_state(
            |ui, chain: &mut Chain| {
                let pending = table::show(ui, &chain.state, &mut chain.table);
                chain.pending.extend(pending);
            },
            chain,
        );
    harness.run();
    assert!(harness.query_all_by_label("Renamed").next().is_some());
}

#[test]
fn hidden_object_is_not_pickable_in_the_viewport() {
    let mut chain = Chain::new();
    let target = ObjectId(1);
    for command in [
        Command::SetPosition(target, Position { x: 0.0, y: 0.0 }),
        Command::SetSize(target, 50.0),
        Command::SetVisible(target, false),
    ] {
        chain.dispatch(command);
    }

    let mut harness = viewport_harness(chain);
    let center = egui::pos2(320.0, 240.0);
    click(&mut harness, center);
    harness.run();
    flush(&mut harness);
    harness.run();

    assert!(harness.state().state.selection().is_empty());
}

#[test]
fn wheel_over_the_viewport_zooms_the_camera() {
    let mut harness = viewport_harness(Chain::new());
    harness.hover_at(egui::pos2(320.0, 240.0));
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: egui::vec2(0.0, 1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: Modifiers::default(),
    });
    harness.run();

    assert!(harness.state().viewport.camera.zoom() > 1.0);
}

#[test]
fn explorer_scrolls_to_the_last_object() {
    for count in starter_core::TEST_LOADS {
        let mut chain = Chain::new();
        chain.dispatch(Command::LoadTestScene(count));
        let last = chain
            .state
            .document()
            .objects()
            .iter()
            .rev()
            .find(|object| object.group() == starter_core::GroupId::C)
            .expect("every demo scene has group C objects")
            .name()
            .to_owned();
        let mut harness = explorer_harness(chain);

        harness.hover_at(egui::pos2(120.0, 160.0));
        harness.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -100_000.0),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: Modifiers::default(),
        });
        harness.run_steps(100);

        let last = harness
            .query_all_by_label(&last)
            .next()
            .unwrap_or_else(|| panic!("last object is not reachable with {count} objects"));
        let rect = last.rect();
        assert!(rect.top() >= 0.0);
        assert!(rect.bottom() <= 320.0);
    }
}

#[test]
fn inspector_follows_an_external_rename_while_unfocused() {
    let mut chain = Chain::new();
    chain.dispatch(Command::Select([ObjectId(1)].into()));
    let mut harness = inspector_harness(chain);

    harness
        .state_mut()
        .dispatch(Command::SetName(ObjectId(1), "External".to_owned()));
    harness.run();

    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some("External")
    );
}

#[test]
fn invalid_draft_survives_an_external_rename() {
    let mut chain = Chain::new();
    chain.dispatch(Command::Select([ObjectId(1)].into()));
    let mut harness = inspector_harness(chain);
    type_name(&mut harness, "   ");

    harness
        .state_mut()
        .dispatch(Command::SetName(ObjectId(1), "External".to_owned()));
    harness.run();

    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some("   ")
    );
    assert_eq!(
        harness.state().state.object(ObjectId(1)).unwrap().name(),
        "External"
    );
}

#[test]
fn switching_selection_resets_the_name_draft() {
    let mut chain = Chain::new();
    chain.dispatch(Command::Select([ObjectId(1)].into()));
    let mut harness = inspector_harness(chain);
    type_name(&mut harness, "   ");

    harness
        .state_mut()
        .dispatch(Command::Select([ObjectId(2)].into()));
    harness.run();

    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some("Object 2")
    );
    assert_eq!(
        harness.state().inspector.name_draft.as_ref().unwrap().0,
        ObjectId(2)
    );
}

#[test]
fn successful_save_syncs_the_draft_to_the_trimmed_name() {
    let mut chain = Chain::new();
    chain.dispatch(Command::Select([ObjectId(1)].into()));
    let mut harness = inspector_harness(chain);

    type_name(&mut harness, "  Renamed  ");

    assert_eq!(
        harness.get_by_role(Role::TextInput).value().as_deref(),
        Some("Renamed")
    );
}
