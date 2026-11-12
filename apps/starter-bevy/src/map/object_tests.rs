use super::{objects::*, real_scene::RealScene, region::ActiveMapRegion};
use crate::project::{Project, ProjectIo};
use bevy::{ecs::schedule::Schedule, prelude::*};
use starter_core::{ProjectSelection, ProjectState};
use std::sync::Arc;
#[test]
fn ecs_projection_reuses_entity_updates_and_releases_materials_without_mutating_document() {
    let dir = super::dem_tests::package(3, 3);
    let data = Arc::new(super::source::load_terrain(dir.path()).unwrap());
    let region = data.manifest.region();
    let mut project = ProjectState::default();
    let id = project.add_marker(region.bounds.center()).unwrap();
    let mut active = ActiveMapRegion::default();
    active.confirm(region).unwrap();
    let mut world = World::new();
    world.insert_resource(Project(project));
    world.insert_resource(ProjectIo::default());
    world.insert_resource(active);
    world.insert_resource(RealScene {
        data: Some(data.clone()),
        revision: Some(1),
        ..Default::default()
    });
    world.init_resource::<MarkerScene>();
    world.init_resource::<Assets<Mesh>>();
    world.init_resource::<Assets<StandardMaterial>>();
    let mut schedule = Schedule::default();
    schedule.add_systems(sync);
    schedule.run(&mut world);
    let entity = world.resource::<MarkerScene>().entities[&id];
    let before = *world.get::<Transform>(entity).unwrap();
    let mut marker = world.resource::<Project>().document.objects[0].clone();
    marker.altitude_agl = 10.0;
    world.resource_mut::<Project>().edit_marker(marker).unwrap();
    schedule.run(&mut world);
    assert_eq!(world.resource::<MarkerScene>().entities[&id], entity);
    assert!(
        (world.get::<Transform>(entity).unwrap().translation.y - before.translation.y - 10.0).abs()
            < 0.001
    );
    let p = world.get::<Transform>(entity).unwrap().translation;
    let ray = crate::viewport::picking::Ray {
        origin: p + Vec3::Y * 100.0,
        direction: -Vec3::Y,
    };
    assert_eq!(
        pick(&world.resource::<Project>().0, &data, &ray, 5.0),
        Some(id)
    );
    for revision in 2..5 {
        world
            .resource_mut::<ActiveMapRegion>()
            .confirm(region)
            .unwrap();
        schedule.run(&mut world);
        assert!(world.resource::<MarkerScene>().entities.is_empty());
        assert_eq!(world.resource::<Assets<StandardMaterial>>().len(), 0);
        world.resource_mut::<RealScene>().revision = Some(revision);
        schedule.run(&mut world);
        assert_eq!(world.resource::<MarkerScene>().entities.len(), 1);
        assert_eq!(world.resource::<Assets<StandardMaterial>>().len(), 1);
        assert_eq!(world.resource::<Assets<Mesh>>().len(), 1);
    }
    assert_eq!(
        world.resource::<Project>().selection,
        ProjectSelection::Marker(id)
    );
    assert_eq!(
        world.resource::<Project>().document.objects[0].altitude_agl,
        10.0
    );
}
#[test]
fn walkers_click_places_and_selects_the_document_marker_offline() {
    use bevy_egui::egui::{self, Event, Modifiers, PointerButton, RawInput, pos2, vec2};
    let ctx = egui::Context::default();
    let mut picker = super::picker::MapPicker::default();
    let mut project = ProjectState {
        placing_marker: true,
        ..Default::default()
    };
    let events = |pressed| Event::PointerButton {
        pos: pos2(400.0, 300.0),
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let mut frame = |events: Vec<Event>, project: &mut ProjectState| {
        ctx.run_ui(
            RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    pos2(0.0, 0.0),
                    vec2(800.0, 600.0),
                )),
                events,
                ..Default::default()
            },
            |ui| picker.show_project_map(ui, None, project),
        )
        .drop_without_applying_deltas();
    };
    frame(vec![], &mut project);
    frame(
        vec![Event::PointerMoved(pos2(400.0, 300.0)), events(true)],
        &mut project,
    );
    frame(vec![events(false)], &mut project);
    assert_eq!(project.document.objects.len(), 1);
    let id = project.document.objects[0].id;
    assert_eq!(project.selection, ProjectSelection::Marker(id));
    project.selection = ProjectSelection::Project;
    frame(vec![events(true)], &mut project);
    frame(vec![events(false)], &mut project);
    assert_eq!(project.selection, ProjectSelection::Marker(id));
    assert_eq!(project.document.objects.len(), 1);
}
