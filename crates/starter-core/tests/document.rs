use std::collections::BTreeSet;

use starter_core::{Command, Document, DocumentState, GroupId, ObjectId, Position, TEST_LOADS};

#[test]
fn demo_load_sizes_and_json_round_trip() {
    for size in TEST_LOADS {
        let document = Document::demo(size).unwrap();
        let json = serde_json::to_string(&document).unwrap();
        let restored: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.objects().len(), size);
        restored.validate().unwrap();
    }
}

#[test]
fn commands_update_one_source_of_truth_and_selection() {
    let mut state = DocumentState::default();
    let id = state
        .dispatch(Command::AddObject(GroupId::A))
        .unwrap()
        .unwrap();
    assert_eq!(state.selection(), &BTreeSet::from([id]));
    state
        .dispatch(Command::SetPosition(id, Position { x: 4.0, y: -3.0 }))
        .unwrap();
    assert_eq!(state.object(id).unwrap().position().x, 4.0);
    state.dispatch(Command::DeleteSelection).unwrap();
    assert!(state.object(id).is_none());
    assert!(state.selection().is_empty());
}

#[test]
fn invalid_positions_and_unknown_schema_are_rejected() {
    let mut state = DocumentState::default();
    let id = ObjectId(1);
    assert!(
        state
            .dispatch(Command::SetPosition(
                id,
                Position {
                    x: f64::NAN,
                    y: 0.0
                },
            ))
            .is_err()
    );

    assert!(
        state
            .dispatch(Command::SetPosition(
                id,
                Position {
                    x: starter_core::MAX_WORLD_COORDINATE * 2.0,
                    y: 0.0,
                },
            ))
            .is_err()
    );
    assert!(
        state
            .dispatch(Command::SetSize(id, starter_core::MAX_OBJECT_SIZE * 2.0))
            .is_err()
    );

    let mut json = serde_json::to_value(Document::empty()).unwrap();
    json["schema_version"] = 99.into();
    let document: Document = serde_json::from_value(json).unwrap();
    assert!(document.validate().is_err());
}

#[test]
fn projection_revisions_only_change_for_relevant_fields() {
    let mut state = DocumentState::default();
    let id = ObjectId(1);
    let structure = state.structure_revision();
    let names = state.name_revision();
    let positions = state.position_revision();

    state
        .dispatch(Command::SetPosition(id, Position { x: 1.0, y: 2.0 }))
        .unwrap();
    assert_eq!(state.structure_revision(), structure);
    assert_eq!(state.name_revision(), names);
    assert_eq!(state.position_revision(), positions + 1);

    state
        .dispatch(Command::SetName(id, "Renamed".to_owned()))
        .unwrap();
    assert_eq!(state.name_revision(), names + 1);
}

#[test]
fn bulk_selection_and_deletion_keep_the_id_index_consistent() {
    let mut state = DocumentState::default();
    state
        .dispatch(Command::Select(BTreeSet::from([
            ObjectId(50),
            ObjectId(75),
            ObjectId(100_000),
        ])))
        .unwrap();
    assert_eq!(
        state.selection(),
        &BTreeSet::from([ObjectId(50), ObjectId(75)])
    );
    state.dispatch(Command::DeleteSelection).unwrap();
    assert!(state.object(ObjectId(50)).is_none());
    assert_eq!(state.object(ObjectId(100)).unwrap().name(), "Object 100");
    let added = state
        .dispatch(Command::AddObject(GroupId::A))
        .unwrap()
        .unwrap();
    assert_eq!(state.object(added).unwrap().name(), "Object 101");
}
