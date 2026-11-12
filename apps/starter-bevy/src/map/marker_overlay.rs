use bevy_egui::egui::{self, Align2, Color32, FontId};
use starter_core::{GeoOrigin, ProjectSelection, ProjectState};
use walkers::Projector;
pub fn show(
    ui: &egui::Ui,
    response: &egui::Response,
    projector: &Projector,
    project: &mut ProjectState,
    selecting: bool,
) {
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let clicked = response.clicked() && !selecting;
    let mut hit = None;
    for marker in project.document.objects.iter().filter(|m| m.visible) {
        let pos = projector
            .project(walkers::lon_lat(marker.longitude, marker.latitude))
            .to_pos2();
        if !response.rect.expand(12.0).contains(pos) {
            continue;
        }
        let selected = project.selection == ProjectSelection::Marker(marker.id);
        let color = Color32::from_rgba_unmultiplied(
            marker.color[0],
            marker.color[1],
            marker.color[2],
            marker.color[3],
        );
        ui.painter().with_clip_rect(response.rect).circle_filled(
            pos,
            if selected { 7.0 } else { 5.0 },
            color,
        );
        ui.painter().with_clip_rect(response.rect).text(
            pos + egui::vec2(9.0, -8.0),
            Align2::LEFT_BOTTOM,
            &marker.name,
            FontId::proportional(12.0),
            Color32::WHITE,
        );
        if clicked && pointer.is_some_and(|p| p.distance(pos) < 12.0) {
            hit = Some(marker.id);
        }
    }
    if clicked {
        if let Some(id) = hit {
            project.selection = ProjectSelection::Marker(id);
        } else if project.placing_marker
            && let Some(p) = pointer.filter(|p| response.rect.contains(*p))
        {
            let point = projector.unproject(p.to_vec2());
            let _ = project.add_marker(GeoOrigin {
                longitude: point.x(),
                latitude: point.y(),
            });
        }
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        project.placing_marker = false;
    }
}
