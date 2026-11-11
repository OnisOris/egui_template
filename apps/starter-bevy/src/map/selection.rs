use super::region::{GeoBounds, GeoOrigin};
use bevy_egui::egui::{self, Color32, PointerButton, Rect, Response, Stroke, StrokeKind, Ui};
use walkers::{Position, Projector, lon_lat};

#[derive(Default)]
pub struct RegionSelectionState {
    pub preview: Option<GeoBounds>,
    pub origin: Option<GeoOrigin>,
    pub cursor: Option<GeoOrigin>,
    pub drag_start: Option<Position>,
    pub error: Option<String>,
}

impl RegionSelectionState {
    pub fn clear(&mut self) {
        self.preview = None;
        self.origin = None;
        self.drag_start = None;
        self.error = None;
    }

    pub fn overlay(
        &mut self,
        ui: &mut Ui,
        response: &Response,
        projector: &Projector,
        select: bool,
    ) {
        let pointer = ui.input(|i| i.pointer.interact_pos());
        self.cursor = pointer.filter(|p| response.rect.contains(*p)).map(|p| {
            let p = projector.unproject(p.to_vec2());
            GeoOrigin {
                longitude: p.x(),
                latitude: p.y(),
            }
        });
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.clear();
            return;
        }
        if select
            && response.hovered()
            && ui.input(|i| i.pointer.button_pressed(PointerButton::Primary))
            && let Some(pointer) = pointer
        {
            self.clear();
            self.drag_start = Some(projector.unproject(pointer.to_vec2()));
        }
        if let Some(start) = self.drag_start {
            if let Some(pointer) = pointer {
                let end = projector.unproject(response.rect.clamp(pointer).to_vec2());
                match rectangle(start, end) {
                    Ok(bounds) => {
                        self.preview = Some(bounds);
                        self.error = None;
                    }
                    Err(error) => {
                        self.preview = None;
                        self.error =
                            (!ui.input(|i| i.pointer.primary_down())).then(|| error.into());
                    }
                }
            }
            if !ui.input(|i| i.pointer.primary_down()) {
                self.drag_start = None;
            }
        }
        if let Some(bounds) = self.preview {
            paint_bounds(ui, projector, bounds, Color32::from_rgb(70, 182, 255));
        }
    }
}

pub fn rectangle(a: Position, b: Position) -> Result<GeoBounds, &'static str> {
    // Validate raw longitudes before ordering: never silently wrap across the world edge.
    GeoOrigin {
        longitude: a.x(),
        latitude: a.y(),
    }
    .validate()?;
    GeoOrigin {
        longitude: b.x(),
        latitude: b.y(),
    }
    .validate()?;
    let bounds = GeoBounds {
        west: a.x().min(b.x()),
        east: a.x().max(b.x()),
        south: a.y().min(b.y()),
        north: a.y().max(b.y()),
    };
    bounds.validate()?;
    Ok(bounds)
}

pub fn paint_bounds(ui: &Ui, projector: &Projector, bounds: GeoBounds, color: Color32) {
    let rect = Rect::from_two_pos(
        projector
            .project(lon_lat(bounds.west, bounds.north))
            .to_pos2(),
        projector
            .project(lon_lat(bounds.east, bounds.south))
            .to_pos2(),
    );
    ui.painter().with_clip_rect(ui.max_rect()).rect(
        rect,
        0.0,
        color.gamma_multiply(0.16),
        Stroke::new(2.0, color),
        StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, Vec2};
    use walkers::{MapMemory, Projector};

    #[test]
    fn geographic_selection_survives_zoom_and_reverse_drag() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let mut memory = MapMemory::default();
        let center = lon_lat(30.0, 60.0);
        let a = lon_lat(29.99, 60.01);
        let b = lon_lat(30.01, 59.99);
        for zoom in [4.0, 12.0, 19.0] {
            memory.set_zoom(zoom).unwrap();
            let p = Projector::new(rect, &memory, center);
            let bounds = rectangle(p.unproject(p.project(b)), p.unproject(p.project(a))).unwrap();
            assert!((bounds.west - a.x()).abs() < 1e-4);
            assert!((bounds.north - a.y()).abs() < 1e-4);
        }
        assert!(rectangle(a, a).is_err());
        assert!(rectangle(lon_lat(179.0, 0.0), lon_lat(181.0, 1.0)).is_err());
    }

    #[test]
    fn cancel_only_clears_temporary_state() {
        let mut state = RegionSelectionState {
            preview: Some(rectangle(lon_lat(1.0, 1.0), lon_lat(2.0, 2.0)).unwrap()),
            drag_start: Some(lon_lat(1.0, 1.0)),
            ..Default::default()
        };
        state.clear();
        assert!(state.preview.is_none() && state.drag_start.is_none());
    }
}
