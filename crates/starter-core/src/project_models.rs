//! Saved model parameters and read-only views over both geographic object types.
use crate::{GeoOrigin, ObjectId, ProjectSelection, ProjectState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModelInstance {
    pub id: ObjectId,
    pub name: String,
    pub asset_path: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_agl: f64,
    pub yaw_degrees: f32,
    pub scale: f32,
    pub visible: bool,
}
impl ModelInstance {
    pub fn validate(&self) -> Result<(), String> {
        GeoOrigin {
            longitude: self.longitude,
            latitude: self.latitude,
        }
        .validate()?;
        if self.id.0 == 0
            || self.name.trim().is_empty()
            || self.name.len() > 256
            || !crate::project::relative_path(&self.asset_path)
            || !self.asset_path.to_lowercase().ends_with(".glb")
            || !self.altitude_agl.is_finite()
            || self.altitude_agl.abs() > 100_000.0
            || !self.yaw_degrees.is_finite()
            || self.yaw_degrees.abs() > 360_000.0
            || !self.scale.is_finite()
            || !(0.001..=1000.0).contains(&self.scale)
        {
            return Err("Invalid model name, path, position, yaw or scale".into());
        }
        Ok(())
    }
}
impl ProjectState {
    pub fn add_model(&mut self, asset_path: String, point: GeoOrigin) -> Result<ObjectId, String> {
        let id = ObjectId(self.document.next_id);
        let model = ModelInstance {
            id,
            name: format!("Model {}", id.0),
            asset_path,
            latitude: point.latitude,
            longitude: point.longitude,
            altitude_agl: 0.0,
            yaw_degrees: 0.0,
            scale: 1.0,
            visible: true,
        };
        model.validate()?;
        if self.document.object_count() >= 100_000
            || !self
                .document
                .assets
                .iter()
                .any(|a| a.path == model.asset_path)
        {
            return Err("Model requires a registered project asset; object limit is 100000".into());
        }
        self.document.next_id = id.0.checked_add(1).ok_or("Object IDs exhausted")?;
        self.document.models.push(model);
        self.selection = ProjectSelection::Model(id);
        self.placing_model = None;
        self.changed();
        Ok(id)
    }
    pub fn edit_model(&mut self, model: ModelInstance) -> Result<(), String> {
        model.validate()?;
        if !self
            .document
            .assets
            .iter()
            .any(|a| a.path == model.asset_path)
        {
            return Err("Model asset is not registered in the project".into());
        }
        let target = self
            .document
            .models
            .iter_mut()
            .find(|m| m.id == model.id)
            .ok_or("Model not found")?;
        if *target != model {
            *target = model;
            self.changed();
        }
        Ok(())
    }
}

/// Borrowed projection, never a second editable document or an ECS identifier.
pub struct ObjectRow<'a> {
    pub id: ObjectId,
    pub name: &'a str,
    pub kind: &'static str,
    pub longitude: f64,
    pub latitude: f64,
    pub altitude_agl: f64,
    pub visible: bool,
    pub color: [u8; 4],
    pub selection: ProjectSelection,
}
impl crate::ProjectDocument {
    pub fn object_count(&self) -> usize {
        self.objects.len() + self.models.len()
    }
    pub fn object_at(&self, index: usize) -> Option<ObjectRow<'_>> {
        if let Some(m) = self.objects.get(index) {
            Some(ObjectRow {
                id: m.id,
                name: &m.name,
                kind: "Marker",
                longitude: m.longitude,
                latitude: m.latitude,
                altitude_agl: m.altitude_agl,
                visible: m.visible,
                color: m.color,
                selection: ProjectSelection::Marker(m.id),
            })
        } else {
            self.models
                .get(index.checked_sub(self.objects.len())?)
                .map(|m| ObjectRow {
                    id: m.id,
                    name: &m.name,
                    kind: "Model",
                    longitude: m.longitude,
                    latitude: m.latitude,
                    altitude_agl: m.altitude_agl,
                    visible: m.visible,
                    color: [80, 185, 245, 255],
                    selection: ProjectSelection::Model(m.id),
                })
        }
    }
    pub fn object_rows(&self) -> impl Iterator<Item = ObjectRow<'_>> {
        (0..self.object_count()).filter_map(|i| self.object_at(i))
    }
    pub fn migrate(&mut self) -> Result<(), String> {
        if self.schema_version == 1 && self.models.is_empty() {
            self.schema_version = 2;
        }
        self.validate()
    }
}
