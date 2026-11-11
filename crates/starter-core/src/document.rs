use std::collections::HashSet;

use serde::{Deserialize, Serialize};

pub const DOCUMENT_SCHEMA_VERSION: u32 = 1;
pub const TEST_LOADS: [usize; 3] = [100, 1_000, 10_000];
pub const MAX_NAME_LENGTH: usize = 256;
pub const MAX_WORLD_COORDINATE: f64 = 1_000_000_000.0;
pub const MAX_OBJECT_SIZE: f32 = 200.0;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectId(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum GroupId {
    A,
    B,
    C,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneObject {
    id: ObjectId,
    pub(crate) name: String,
    group: GroupId,
    pub(crate) position: Position,
    pub(crate) size: f32,
    pub(crate) color: [u8; 4],
    pub(crate) visible: bool,
    pub(crate) enabled: bool,
}

impl SceneObject {
    pub fn id(&self) -> ObjectId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn group(&self) -> GroupId {
        self.group
    }

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn size(&self) -> f32 {
        self.size
    }

    pub fn color(&self) -> [u8; 4] {
        self.color
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    schema_version: u32,
    objects: Vec<SceneObject>,
    next_id: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DocumentError {
    UnsupportedVersion(u32),
    DuplicateId(ObjectId),
    InvalidObject(ObjectId),
    IdExhausted,
    InvalidTestLoad(usize),
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DocumentError {}

impl Document {
    pub fn empty() -> Self {
        Self {
            schema_version: DOCUMENT_SCHEMA_VERSION,
            objects: Vec::new(),
            next_id: 1,
        }
    }

    pub fn demo(count: usize) -> Result<Self, DocumentError> {
        if !TEST_LOADS.contains(&count) {
            return Err(DocumentError::InvalidTestLoad(count));
        }
        let columns = (count as f64).sqrt().ceil() as usize;
        let rows = count.div_ceil(columns);
        let objects = (1..=count)
            .map(|number| {
                let id = ObjectId(number as u64);
                let column = (number - 1) % columns;
                let row = (number - 1) / columns;
                SceneObject {
                    id,
                    name: format!("Object {number}"),
                    group: match number % 3 {
                        0 => GroupId::A,
                        1 => GroupId::B,
                        _ => GroupId::C,
                    },
                    position: Position {
                        x: (column as f64 - (columns - 1) as f64 / 2.0) * 32.0,
                        y: ((rows - 1) as f64 / 2.0 - row as f64) * 32.0,
                    },
                    size: 8.0 + (number % 5) as f32 * 1.5,
                    color: demo_color(number),
                    visible: true,
                    enabled: true,
                }
            })
            .collect();
        Ok(Self {
            schema_version: DOCUMENT_SCHEMA_VERSION,
            objects,
            next_id: count as u64 + 1,
        })
    }

    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub fn objects(&self) -> &[SceneObject] {
        &self.objects
    }

    pub fn object(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects.iter().find(|object| object.id == id)
    }

    pub fn validate(&self) -> Result<(), DocumentError> {
        if self.schema_version != DOCUMENT_SCHEMA_VERSION {
            return Err(DocumentError::UnsupportedVersion(self.schema_version));
        }
        let mut ids = HashSet::with_capacity(self.objects.len());
        for object in &self.objects {
            if !ids.insert(object.id) {
                return Err(DocumentError::DuplicateId(object.id));
            }
            if !object.position.x.is_finite()
                || !object.position.y.is_finite()
                || object.position.x.abs() > MAX_WORLD_COORDINATE
                || object.position.y.abs() > MAX_WORLD_COORDINATE
                || !object.size.is_finite()
                || object.size <= 0.0
                || object.size > MAX_OBJECT_SIZE
                || object.name.trim().is_empty()
                || object.name.len() > MAX_NAME_LENGTH
            {
                return Err(DocumentError::InvalidObject(object.id));
            }
        }
        if self.next_id == 0
            || self
                .objects
                .iter()
                .any(|object| object.id.0 >= self.next_id)
        {
            return Err(DocumentError::IdExhausted);
        }
        Ok(())
    }

    pub(crate) fn object_at_mut(&mut self, index: usize) -> Option<&mut SceneObject> {
        self.objects.get_mut(index)
    }

    pub(crate) fn add(&mut self, group: GroupId) -> Result<ObjectId, DocumentError> {
        let id = ObjectId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(DocumentError::IdExhausted)?;
        self.objects.push(SceneObject {
            id,
            name: format!("Object {}", id.0),
            group,
            position: Position::default(),
            size: 10.0,
            color: demo_color(id.0 as usize),
            visible: true,
            enabled: true,
        });
        Ok(id)
    }

    pub(crate) fn delete_many(&mut self, ids: &HashSet<ObjectId>) {
        self.objects.retain(|object| !ids.contains(&object.id));
    }
}

fn demo_color(id: usize) -> [u8; 4] {
    let hue = id.wrapping_mul(47) % 180;
    [
        70 + (hue / 2) as u8,
        130 + (hue / 3) as u8,
        210 - (hue / 3) as u8,
        255,
    ]
}
