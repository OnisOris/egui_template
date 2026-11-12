use std::collections::{BTreeSet, HashMap};
use std::time::Instant;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use starter_core::{ObjectId, Position};

use crate::state::DocumentResource;

use crate::metrics::EditorMetrics;

use super::objects::{ProjectedObject, SceneAssets, object_transform};

#[derive(Clone, Copy, Debug)]
pub struct ProjectionEntry {
    pub entity: Entity,
    pub position: Position,
    pub size: f32,
    pub color: [u8; 4],
    pub visible: bool,
}

#[derive(Resource)]
pub struct SceneSync {
    pub entries: HashMap<ObjectId, ProjectionEntry>,
    structure: u64,
    position: u64,
    size: u64,
    color: u64,
    visible: u64,
}

impl Default for SceneSync {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            structure: u64::MAX,
            position: u64::MAX,
            size: u64::MAX,
            color: u64::MAX,
            visible: u64::MAX,
        }
    }
}

pub fn sync_scene(
    mut commands: Commands,
    document: Res<DocumentResource>,
    mut sync: ResMut<SceneSync>,
    mut assets: ResMut<SceneAssets>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut metrics: ResMut<EditorMetrics>,
) {
    let changed = sync.structure != document.structure_revision()
        || sync.position != document.position_revision()
        || sync.size != document.size_revision()
        || sync.color != document.color_revision()
        || sync.visible != document.visible_revision();
    if !changed {
        return;
    }

    let started = Instant::now();
    let structure_changed = sync.structure != document.structure_revision();
    if structure_changed {
        let current: BTreeSet<_> = document
            .document()
            .objects()
            .iter()
            .map(|object| object.id())
            .collect();
        sync.entries.retain(|id, entry| {
            if current.contains(id) {
                true
            } else {
                commands.entity(entry.entity).despawn();
                false
            }
        });
    }

    for object in document.document().objects() {
        let id = object.id();
        if let std::collections::hash_map::Entry::Vacant(slot) = sync.entries.entry(id) {
            let entity = commands
                .spawn((
                    Mesh3d(assets.mesh(object)),
                    MeshMaterial3d(assets.material(&mut materials, object.color())),
                    object_transform(object),
                    visibility(object.visible()),
                    RenderLayers::layer(1),
                    ProjectedObject(id),
                ))
                .id();
            slot.insert(ProjectionEntry {
                entity,
                position: object.position(),
                size: object.size(),
                color: object.color(),
                visible: object.visible(),
            });
            continue;
        }

        let entry = sync.entries.get_mut(&id).expect("entry checked above");
        let mut entity = commands.entity(entry.entity);
        if entry.position != object.position() || entry.size != object.size() {
            entity.insert(object_transform(object));
        }
        if entry.color != object.color() {
            entity.insert(MeshMaterial3d(
                assets.material(&mut materials, object.color()),
            ));
        }
        if entry.visible != object.visible() {
            entity.insert(visibility(object.visible()));
        }
        entry.position = object.position();
        entry.size = object.size();
        entry.color = object.color();
        entry.visible = object.visible();
    }

    sync.structure = document.structure_revision();
    sync.position = document.position_revision();
    sync.size = document.size_revision();
    sync.color = document.color_revision();
    sync.visible = document.visible_revision();
    metrics.projected_entities = sync.entries.len();
    metrics.objects = document.document().objects().len();
    metrics.sync_ms = started.elapsed().as_secs_f32() * 1000.0;
    if structure_changed {
        metrics.scene_build_ms = metrics.sync_ms;
    }
}

fn visibility(visible: bool) -> Visibility {
    if visible {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::schedule::Schedule;
    use starter_core::Command;

    use super::*;

    fn test_world(document: DocumentResource) -> World {
        let mut world = World::new();
        world.insert_resource(document);
        world.insert_resource(Assets::<Mesh>::default());
        world.insert_resource(Assets::<StandardMaterial>::default());
        world.init_resource::<SceneAssets>();
        world.init_resource::<SceneSync>();
        world.init_resource::<EditorMetrics>();
        world
    }

    fn run_sync(world: &mut World) {
        let mut schedule = Schedule::default();
        schedule.add_systems(sync_scene);
        schedule.run(world);
    }

    #[test]
    fn projection_is_stable_when_position_changes() {
        let mut world = test_world(DocumentResource(starter_core::DocumentState::default()));
        run_sync(&mut world);
        let before = world.resource::<SceneSync>().entries[&ObjectId(1)].entity;

        world
            .resource_mut::<DocumentResource>()
            .dispatch(Command::SetPosition(
                ObjectId(1),
                Position { x: 8.0, y: 9.0 },
            ))
            .unwrap();
        run_sync(&mut world);

        let after = world.resource::<SceneSync>().entries[&ObjectId(1)].entity;
        assert_eq!(before, after);
        assert_eq!(
            world.resource::<SceneSync>().entries[&ObjectId(1)].position,
            Position { x: 8.0, y: 9.0 }
        );
    }

    #[test]
    fn deleting_an_object_despawns_it() {
        let mut world = test_world(DocumentResource(starter_core::DocumentState::default()));
        run_sync(&mut world);
        let entity = world.resource::<SceneSync>().entries[&ObjectId(1)].entity;

        world
            .resource_mut::<DocumentResource>()
            .dispatch(Command::Select([ObjectId(1)].into()))
            .unwrap();
        world
            .resource_mut::<DocumentResource>()
            .dispatch(Command::DeleteSelection)
            .unwrap();
        run_sync(&mut world);

        assert!(
            !world
                .resource::<SceneSync>()
                .entries
                .contains_key(&ObjectId(1))
        );
        assert!(world.get_entity(entity).is_err());
    }

    #[test]
    fn adding_an_object_creates_one_entity() {
        let mut world = test_world(DocumentResource(starter_core::DocumentState::default()));
        run_sync(&mut world);

        world
            .resource_mut::<DocumentResource>()
            .dispatch(Command::AddObject(starter_core::GroupId::A))
            .unwrap();
        run_sync(&mut world);

        assert_eq!(world.resource::<SceneSync>().entries.len(), 101);
        assert_eq!(world.query::<&ProjectedObject>().iter(&world).count(), 101);
    }

    #[test]
    fn switching_load_rebuilds_the_projection() {
        let mut world = test_world(DocumentResource(starter_core::DocumentState::default()));
        run_sync(&mut world);

        world
            .resource_mut::<DocumentResource>()
            .dispatch(Command::LoadTestScene(1_000))
            .unwrap();
        run_sync(&mut world);

        assert_eq!(world.resource::<SceneSync>().entries.len(), 1_000);
        assert_eq!(
            world.query::<&ProjectedObject>().iter(&world).count(),
            1_000
        );
    }
    #[test]
    fn ten_thousand_demo_entities_still_project_with_a_bounded_asset_palette() {
        let mut doc = DocumentResource::empty();
        doc.dispatch(Command::LoadTestScene(10_000)).unwrap();
        let mut world = test_world(doc);
        run_sync(&mut world);
        assert_eq!(world.resource::<SceneSync>().entries.len(), 10_000);
        println!(
            "M4 CPU benchmark: 10000 entities, {:.3} ms",
            world.resource::<EditorMetrics>().scene_build_ms
        );
        assert!(world.resource::<Assets<StandardMaterial>>().len() <= 180);
    }
}
