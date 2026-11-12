//! Saved project data, independent of UI and ECS handles.
use crate::{GeoOrigin, MapRegion, ObjectId};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Component, Path},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LayerKind {
    Terrain,
    Satellite,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapLayer {
    pub kind: LayerKind,
    pub visible: bool,
    pub package_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MapMarker {
    pub id: ObjectId,
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_agl: f64,
    pub visible: bool,
    pub color: [u8; 4],
}
impl MapMarker {
    pub fn validate(&self) -> Result<(), String> {
        GeoOrigin {
            longitude: self.longitude,
            latitude: self.latitude,
        }
        .validate()?;
        if self.id.0 == 0
            || self.name.trim().is_empty()
            || self.name.len() > 256
            || !self.altitude_agl.is_finite()
            || self.altitude_agl.abs() > 100_000.0
        {
            return Err("Invalid marker name, ID or altitude".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectAsset {
    pub path: String,
    pub media_type: String,
    pub size: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProjectDocument {
    pub schema_version: u32,
    pub name: String,
    pub map_region: Option<MapRegion>,
    pub layers: Vec<MapLayer>,
    pub objects: Vec<MapMarker>,
    pub assets: Vec<ProjectAsset>,
    #[serde(default)]
    pub models: Vec<crate::ModelInstance>,
    pub(crate) next_id: u64,
}
impl Default for ProjectDocument {
    fn default() -> Self {
        Self {
            schema_version: 2,
            name: "Untitled Project".into(),
            map_region: None,
            objects: Vec::new(),
            models: Vec::new(),
            assets: Vec::new(),
            next_id: 1,
            layers: [LayerKind::Terrain, LayerKind::Satellite]
                .into_iter()
                .map(|kind| MapLayer {
                    kind,
                    visible: true,
                    package_id: None,
                })
                .collect(),
        }
    }
}
impl ProjectDocument {
    pub fn layer(&self, kind: LayerKind) -> &MapLayer {
        self.layers
            .iter()
            .find(|l| l.kind == kind)
            .expect("validated project layers")
    }
    pub fn layer_mut(&mut self, kind: LayerKind) -> &mut MapLayer {
        self.layers
            .iter_mut()
            .find(|l| l.kind == kind)
            .expect("validated project layers")
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 2 {
            return Err("Unsupported project schema version".into());
        }
        if self.name.trim().is_empty()
            || self.name.len() > 256
            || self.object_count() > 100_000
            || self.assets.len() > 10_000
        {
            return Err("Invalid project name or item count".into());
        }
        if let Some(r) = self.map_region {
            MapRegion::new(r.bounds, r.origin)?;
        }
        if self.layers.len() != 2
            || [LayerKind::Terrain, LayerKind::Satellite]
                .iter()
                .any(|k| self.layers.iter().filter(|l| l.kind == *k).count() != 1)
        {
            return Err("Project requires unique Terrain and Satellite layers".into());
        }
        for layer in &self.layers {
            if layer.package_id.as_ref().is_some_and(|id| {
                id.len() > 256
                    || id.is_empty()
                    || !id
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            }) {
                return Err("Invalid geographic package ID".into());
            }
        }
        let mut ids = HashSet::new();
        for object in &self.objects {
            object.validate()?;
            if !ids.insert(object.id) || object.id.0 >= self.next_id {
                return Err("Duplicate marker ID or invalid ID sequence".into());
            }
        }
        for model in &self.models {
            model.validate()?;
            if !ids.insert(model.id)
                || model.id.0 >= self.next_id
                || !self.assets.iter().any(|a| a.path == model.asset_path)
            {
                return Err(
                    "Duplicate object ID, invalid sequence or unregistered model asset".into(),
                );
            }
        }
        if self.next_id == 0 {
            return Err("Invalid marker ID sequence".into());
        }
        let mut paths = HashSet::new();
        for asset in &self.assets {
            if !relative_path(&asset.path) || !paths.insert(&asset.path) {
                return Err("Invalid or duplicate project asset path".into());
            }
        }
        Ok(())
    }
}
pub fn relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && !path.contains(':')
        && !path.contains('\0')
        && Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum ProjectSelection {
    #[default]
    Project,
    Manifest,
    Layer(LayerKind),
    Marker(ObjectId),
    Model(ObjectId),
    Asset(usize),
}
#[derive(Clone, Debug, Default)]
pub struct ProjectState {
    pub document: ProjectDocument,
    pub selection: ProjectSelection,
    pub revision: u64,
    pub dirty: bool,
    pub placing_marker: bool,
    pub placing_model: Option<String>,
    pub focus_selected: bool,
}
impl ProjectState {
    pub fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.dirty = true;
    }
    pub fn add_marker(&mut self, point: GeoOrigin) -> Result<ObjectId, String> {
        point.validate()?;
        if self.document.object_count() >= 100_000 {
            return Err("Marker limit reached".into());
        }
        let id = ObjectId(self.document.next_id);
        self.document.next_id = id.0.checked_add(1).ok_or("Marker IDs exhausted")?;
        self.document.objects.push(MapMarker {
            id,
            name: format!("Marker {}", id.0),
            latitude: point.latitude,
            longitude: point.longitude,
            altitude_agl: 0.0,
            visible: true,
            color: [240, 170, 55, 255],
        });
        self.selection = ProjectSelection::Marker(id);
        self.placing_marker = false;
        self.changed();
        Ok(id)
    }
    pub fn edit_marker(&mut self, marker: MapMarker) -> Result<(), String> {
        marker.validate()?;
        let target = self
            .document
            .objects
            .iter_mut()
            .find(|m| m.id == marker.id)
            .ok_or("Marker not found")?;
        if *target != marker {
            *target = marker;
            self.changed();
        }
        Ok(())
    }
    pub fn delete_selected(&mut self) {
        if let ProjectSelection::Marker(id) | ProjectSelection::Model(id) = self.selection {
            self.document.objects.retain(|m| m.id != id);
            self.document.models.retain(|m| m.id != id);
            self.selection = ProjectSelection::Project;
            self.changed();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_project_and_roundtrip_preserve_stable_markers_and_layers() {
        let mut state = ProjectState::default();
        assert!(state.document.objects.is_empty());
        let bounds = crate::GeoBounds {
            west: 7.0,
            east: 8.0,
            south: 46.0,
            north: 47.0,
        };
        state.document.map_region = Some(MapRegion::new(bounds, bounds.center()).unwrap());
        state.document.layer_mut(LayerKind::Satellite).visible = false;
        state.document.layer_mut(LayerKind::Terrain).package_id = Some("cached-terrain".into());
        let id = state
            .add_marker(GeoOrigin {
                latitude: 46.0,
                longitude: 8.0,
            })
            .unwrap();
        let mut marker = state.document.objects[0].clone();
        marker.altitude_agl = 12.0;
        state.edit_marker(marker).unwrap();
        let json = serde_json::to_vec(&state.document).unwrap();
        let doc: ProjectDocument = serde_json::from_slice(&json).unwrap();
        doc.validate().unwrap();
        assert_eq!(doc, state.document);
        assert_eq!(doc.objects[0].id, id);
    }
    #[test]
    fn reject_bad_schema_coordinates_and_paths() {
        let mut doc = ProjectDocument {
            schema_version: 99,
            ..Default::default()
        };
        assert!(doc.validate().is_err());
        doc.schema_version = 1;
        doc.assets.push(ProjectAsset {
            path: "../secret".into(),
            media_type: "file".into(),
            size: 0,
        });
        assert!(doc.validate().is_err());
        assert!(!relative_path("/tmp/data"));
        assert!(!relative_path("Assets/../../data"));
        assert!(relative_path("Assets/Textures/test.jpg"));
        assert!(
            ProjectState::default()
                .add_marker(GeoOrigin {
                    latitude: 90.0,
                    longitude: 0.0
                })
                .is_err()
        );
    }
}
