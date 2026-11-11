//! Widget input regression tests, deliberately offline. Not a manual UX certification.
use super::{
    picker::{MapPicker, Tool, initial_center},
    region::{ActiveMapRegion, MapRegion},
};
use bevy_egui::egui::{self, Context, Event, Modifiers, PointerButton, RawInput, Rect, pos2, vec2};
use walkers::Projector;

fn frame(ctx: &Context, picker: &mut MapPicker, events: Vec<Event>) {
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
    let output = ctx.run_ui(
        RawInput {
            screen_rect: Some(rect),
            events,
            ..Default::default()
        },
        |ui| picker.show_map(ui, None),
    );
    output.drop_without_applying_deltas();
}

fn button(x: f32, y: f32, pressed: bool) -> Event {
    Event::PointerButton {
        pos: pos2(x, y),
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn drag_cancel_zoom_and_pan_preserve_confirmed_region() {
    let ctx = Context::default();
    let mut picker = MapPicker::default();
    picker.view.tool = Tool::Select;
    frame(&ctx, &mut picker, vec![]);
    frame(
        &ctx,
        &mut picker,
        vec![Event::PointerMoved(pos2(250.0, 200.0))],
    );
    frame(&ctx, &mut picker, vec![button(250.0, 200.0, true)]);
    frame(
        &ctx,
        &mut picker,
        vec![Event::PointerMoved(pos2(550.0, 400.0))],
    );
    assert!(picker.selection.drag_start.is_some());
    frame(&ctx, &mut picker, vec![button(550.0, 400.0, false)]);
    let bounds = picker.selection.preview.expect("valid drag bbox");
    assert!(
        picker.view.memory.detached().is_none(),
        "select must not pan"
    );
    assert!(picker.selection.drag_start.is_none());
    let mut active = ActiveMapRegion::default();
    active
        .confirm(MapRegion::new(bounds, bounds.center()).unwrap())
        .unwrap();
    let confirmed = active.get();
    let zoom = picker.view.memory.zoom();
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
    let anchor_before =
        Projector::new(rect, &picker.view.memory, initial_center()).unproject(vec2(550.0, 400.0));
    // Wheel smoothing can span multiple frames.
    frame(
        &ctx,
        &mut picker,
        vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            phase: egui::TouchPhase::Move,
            delta: vec2(0.0, 100.0),
            modifiers: Modifiers::NONE,
        }],
    );
    for _ in 0..20 {
        frame(&ctx, &mut picker, vec![]);
    }
    assert!(picker.view.memory.zoom() > zoom);
    let anchor_after =
        Projector::new(rect, &picker.view.memory, initial_center()).unproject(vec2(550.0, 400.0));
    assert!((anchor_before.x() - anchor_after.x()).abs() < 1e-5);
    assert!((anchor_before.y() - anchor_after.y()).abs() < 1e-5);
    assert_eq!(picker.selection.preview, Some(bounds));
    assert_eq!(active.get(), confirmed);
    frame(
        &ctx,
        &mut picker,
        vec![Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(picker.selection.preview.is_none());
    assert_eq!(active.get(), confirmed);
    picker.view.tool = Tool::Pan;
    let before = picker.view.memory.detached();
    frame(&ctx, &mut picker, vec![button(550.0, 400.0, true)]);
    frame(
        &ctx,
        &mut picker,
        vec![Event::PointerMoved(pos2(650.0, 450.0))],
    );
    frame(&ctx, &mut picker, vec![button(650.0, 450.0, false)]);
    assert_ne!(picker.view.memory.detached(), before);
    assert_eq!(active.get(), confirmed);
}

#[test]
fn fit_contains_region_at_projection_edge_and_search_rejects_invalid_input() {
    let mut picker = MapPicker::default();
    let bounds = super::selection::rectangle(
        walkers::lon_lat(179.9, 85.0),
        walkers::lon_lat(180.0, 85.05),
    )
    .unwrap();
    let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(600.0, 400.0));
    picker.view.fit(bounds, rect);
    let p = Projector::new(rect, &picker.view.memory, initial_center());
    for position in [
        walkers::lon_lat(bounds.west, bounds.north),
        walkers::lon_lat(bounds.east, bounds.south),
    ] {
        assert!(rect.contains(p.project(position).to_pos2()));
    }
    for invalid in ["NaN, 30", "90, 30", "50, 181", "not coordinates"] {
        picker.view.search = invalid.into();
        assert!(picker.view.search().is_err());
    }
}
