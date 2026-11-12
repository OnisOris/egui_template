use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{
    Document, DocumentError, GroupId, MAX_NAME_LENGTH, MAX_OBJECT_SIZE, MAX_WORLD_COORDINATE,
    ObjectId, Position, TEST_LOADS,
};

#[derive(Clone, Debug)]
pub enum Command {
    NewDocument,
    LoadTestScene(usize),
    AddObject(GroupId),
    DeleteSelection,
    Select(BTreeSet<ObjectId>),
    ReplaceDocument(Document),
    SetName(ObjectId, String),
    SetPosition(ObjectId, Position),
    SetSize(ObjectId, f32),
    SetColor(ObjectId, [u8; 4]),
    SetVisible(ObjectId, bool),
    SetEnabled(ObjectId, bool),
}

#[derive(Clone, Debug)]
pub struct DocumentState {
    document: Document,
    selection: BTreeSet<ObjectId>,
    object_indices: HashMap<ObjectId, usize>,
    structure_revision: u64,
    name_revision: u64,
    position_revision: u64,
    size_revision: u64,
    color_revision: u64,
    visible_revision: u64,
}

impl Default for DocumentState {
    fn default() -> Self {
        Self::from_document(Document::demo(TEST_LOADS[0]).expect("built-in load is valid"))
    }
}

impl DocumentState {
    pub fn empty() -> Self {
        Self::from_document(Document::empty())
    }
    fn from_document(document: Document) -> Self {
        let object_indices = index_objects(&document);
        Self {
            document,
            selection: BTreeSet::new(),
            object_indices,
            structure_revision: 0,
            name_revision: 0,
            position_revision: 0,
            size_revision: 0,
            color_revision: 0,
            visible_revision: 0,
        }
    }
}

impl DocumentState {
    pub fn document(&self) -> &Document {
        &self.document
    }

    pub fn selection(&self) -> &BTreeSet<ObjectId> {
        &self.selection
    }

    pub fn selected_object(&self) -> Option<&crate::SceneObject> {
        if self.selection.len() == 1 {
            self.object(*self.selection.first()?)
        } else {
            None
        }
    }

    pub fn object(&self, id: ObjectId) -> Option<&crate::SceneObject> {
        self.object_indices
            .get(&id)
            .and_then(|index| self.document.objects().get(*index))
    }

    fn object_mut(&mut self, id: ObjectId) -> Option<&mut crate::SceneObject> {
        let index = *self.object_indices.get(&id)?;
        self.document.object_at_mut(index)
    }

    fn rebuild_index(&mut self) {
        self.object_indices = index_objects(&self.document);
    }

    pub fn structure_revision(&self) -> u64 {
        self.structure_revision
    }

    pub fn name_revision(&self) -> u64 {
        self.name_revision
    }

    pub fn position_revision(&self) -> u64 {
        self.position_revision
    }

    pub fn size_revision(&self) -> u64 {
        self.size_revision
    }

    pub fn color_revision(&self) -> u64 {
        self.color_revision
    }

    pub fn visible_revision(&self) -> u64 {
        self.visible_revision
    }

    pub fn dispatch(&mut self, command: Command) -> Result<Option<ObjectId>, DocumentError> {
        let mut structure_changed = false;
        let mut name_changed = false;
        let mut position_changed = false;
        let mut size_changed = false;
        let mut color_changed = false;
        let mut visible_changed = false;
        let result = match command {
            Command::NewDocument => {
                structure_changed = !self.document.objects().is_empty();
                self.document = Document::empty();
                self.object_indices.clear();
                self.selection.clear();
                None
            }
            Command::LoadTestScene(count) => {
                let document = Document::demo(count)?;
                structure_changed = self.document != document;
                self.document = document;
                self.rebuild_index();
                self.selection.clear();
                None
            }
            Command::AddObject(group) => {
                let id = self.document.add(group)?;
                structure_changed = true;
                self.object_indices
                    .insert(id, self.document.objects().len() - 1);
                self.selection.clear();
                self.selection.insert(id);
                Some(id)
            }
            Command::DeleteSelection => {
                let selected: HashSet<_> =
                    std::mem::take(&mut self.selection).into_iter().collect();
                structure_changed = !selected.is_empty();
                if structure_changed {
                    self.document.delete_many(&selected);
                    self.rebuild_index();
                }
                None
            }
            Command::Select(ids) => {
                let selection = ids
                    .into_iter()
                    .filter(|id| self.object_indices.contains_key(id))
                    .collect();
                self.selection = selection;
                None
            }
            Command::ReplaceDocument(document) => {
                document.validate()?;
                structure_changed = self.document != document;
                self.document = document;
                self.rebuild_index();
                self.selection.clear();
                None
            }
            Command::SetName(id, name) => {
                let name = name.trim().to_owned();
                if name.is_empty() || name.len() > MAX_NAME_LENGTH {
                    return Err(DocumentError::InvalidObject(id));
                }
                let object = self
                    .object_mut(id)
                    .ok_or(DocumentError::InvalidObject(id))?;
                name_changed = object.name != name;
                object.name = name;
                None
            }
            Command::SetPosition(id, position) => {
                if !position.x.is_finite()
                    || !position.y.is_finite()
                    || position.x.abs() > MAX_WORLD_COORDINATE
                    || position.y.abs() > MAX_WORLD_COORDINATE
                {
                    return Err(DocumentError::InvalidObject(id));
                }
                let object = self
                    .object_mut(id)
                    .ok_or(DocumentError::InvalidObject(id))?;
                position_changed = object.position != position;
                object.position = position;
                None
            }
            Command::SetSize(id, size) => {
                if !size.is_finite() || size <= 0.0 || size > MAX_OBJECT_SIZE {
                    return Err(DocumentError::InvalidObject(id));
                }
                let object = self
                    .object_mut(id)
                    .ok_or(DocumentError::InvalidObject(id))?;
                size_changed = object.size != size;
                object.size = size;
                None
            }
            Command::SetColor(id, color) => {
                let object = self
                    .object_mut(id)
                    .ok_or(DocumentError::InvalidObject(id))?;
                color_changed = object.color != color;
                object.color = color;
                None
            }
            Command::SetVisible(id, visible) => {
                let object = self
                    .object_mut(id)
                    .ok_or(DocumentError::InvalidObject(id))?;
                visible_changed = object.visible != visible;
                object.visible = visible;
                None
            }
            Command::SetEnabled(id, enabled) => {
                let object = self
                    .object_mut(id)
                    .ok_or(DocumentError::InvalidObject(id))?;
                object.enabled = enabled;
                None
            }
        };
        if structure_changed {
            self.structure_revision = self.structure_revision.wrapping_add(1);
        }
        if name_changed {
            self.name_revision = self.name_revision.wrapping_add(1);
        }
        if position_changed {
            self.position_revision = self.position_revision.wrapping_add(1);
        }
        if size_changed {
            self.size_revision = self.size_revision.wrapping_add(1);
        }
        if color_changed {
            self.color_revision = self.color_revision.wrapping_add(1);
        }
        if visible_changed {
            self.visible_revision = self.visible_revision.wrapping_add(1);
        }
        Ok(result)
    }
}

fn index_objects(document: &Document) -> HashMap<ObjectId, usize> {
    document
        .objects()
        .iter()
        .enumerate()
        .map(|(index, object)| (object.id(), index))
        .collect()
}
