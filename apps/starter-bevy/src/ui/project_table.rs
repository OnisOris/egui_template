use bevy_egui::egui::{self, Ui};
use starter_core::{ProjectSelection, ProjectState};
use starter_ui::{CellInfo, Column, HeaderCellInfo, TableDelegate, show_table};
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Sort {
    #[default]
    Id,
    Name,
    Latitude,
    Longitude,
    Altitude,
    Visible,
}
#[derive(Default)]
pub struct TableState {
    filter: String,
    sort: Sort,
    descending: bool,
    projection: Option<(u64, String, Sort, bool)>,
    rows: Vec<usize>,
}
impl TableState {
    fn refresh(&mut self, project: &ProjectState) {
        let key = (
            project.revision,
            self.filter.clone(),
            self.sort,
            self.descending,
        );
        if self.projection.as_ref() == Some(&key) {
            return;
        }
        let query = self.filter.to_lowercase();
        self.rows = project
            .document
            .objects
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                query.is_empty()
                    || m.name.to_lowercase().contains(&query)
                    || m.id.0.to_string().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();
        self.rows.sort_by(|a, b| {
            let a = &project.document.objects[*a];
            let b = &project.document.objects[*b];
            let order = match self.sort {
                Sort::Id => a.id.cmp(&b.id),
                Sort::Name => a.name.cmp(&b.name),
                Sort::Latitude => a.latitude.total_cmp(&b.latitude),
                Sort::Longitude => a.longitude.total_cmp(&b.longitude),
                Sort::Altitude => a.altitude_agl.total_cmp(&b.altitude_agl),
                Sort::Visible => a.visible.cmp(&b.visible),
            };
            if self.descending {
                order.reverse()
            } else {
                order
            }
        });
        self.projection = Some(key);
    }
}
pub fn show(ui: &mut Ui, project: &mut ProjectState, state: &mut TableState) {
    ui.horizontal_wrapped(|ui| {
        ui.label("Filter");
        ui.text_edit_singleline(&mut state.filter);
        egui::ComboBox::from_id_salt("marker_sort")
            .selected_text(format!("{:?}", state.sort))
            .show_ui(ui, |ui| {
                for sort in [
                    Sort::Id,
                    Sort::Name,
                    Sort::Latitude,
                    Sort::Longitude,
                    Sort::Altitude,
                    Sort::Visible,
                ] {
                    ui.selectable_value(&mut state.sort, sort, format!("{sort:?}"));
                }
            });
        ui.checkbox(&mut state.descending, "Descending");
    });
    state.refresh(project);
    let mut delegate = Markers {
        project,
        rows: &state.rows,
    };
    show_table(
        ui,
        "map_objects",
        state.rows.len() as u64,
        [72.0, 160.0, 80.0, 115.0, 115.0, 100.0, 70.0]
            .into_iter()
            .map(Column::new)
            .collect(),
        &mut delegate,
    );
}
struct Markers<'a> {
    project: &'a mut ProjectState,
    rows: &'a [usize],
}
impl TableDelegate for Markers<'_> {
    fn header_cell_ui(&mut self, ui: &mut Ui, cell: &HeaderCellInfo) {
        ui.strong(
            [
                "ID",
                "Name",
                "Type",
                "Latitude",
                "Longitude",
                "Altitude AGL",
                "Visible",
            ]
            .get(cell.col_range.start)
            .copied()
            .unwrap_or(""),
        );
    }
    fn cell_ui(&mut self, ui: &mut Ui, cell: &CellInfo) {
        let Some(m) = self
            .rows
            .get(cell.row_nr as usize)
            .and_then(|i| self.project.document.objects.get(*i))
        else {
            return;
        };
        let id = m.id;
        let text = match cell.col_nr {
            0 => id.0.to_string(),
            1 => m.name.clone(),
            2 => "Marker".into(),
            3 => format!("{:.7}", m.latitude),
            4 => format!("{:.7}", m.longitude),
            5 => format!("{:.2}", m.altitude_agl),
            6 => m.visible.to_string(),
            _ => String::new(),
        };
        if ui
            .selectable_label(self.project.selection == ProjectSelection::Marker(id), text)
            .clicked()
        {
            self.project.selection = ProjectSelection::Marker(id);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sorting_and_filtering_reference_the_original_marker_ids() {
        let mut p = ProjectState::default();
        for lon in [8.0, 7.0] {
            p.add_marker(starter_core::GeoOrigin {
                latitude: 46.0,
                longitude: lon,
            })
            .unwrap();
        }
        let mut t = TableState {
            sort: Sort::Longitude,
            ..Default::default()
        };
        t.refresh(&p);
        assert_eq!(t.rows, vec![1, 0]);
        t.filter = "Marker 1".into();
        t.refresh(&p);
        assert_eq!(t.rows, vec![0]);
    }
}
