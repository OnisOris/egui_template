use eframe::egui::{self, Ui};
use starter_core::{Command, DocumentState, ObjectId, SceneObject};
use starter_ui::{CellInfo, Column, HeaderCellInfo, TableDelegate, show_table};

use crate::state::TableSort;

pub struct TableState {
    pub filter: String,
    pub sort: TableSort,
    pub ascending: bool,
    revision: [u64; 3],
    projected_filter: String,
    projected_sort: TableSort,
    projected_ascending: bool,
    row_ids: Vec<ObjectId>,
    dirty: bool,
}

impl Default for TableState {
    fn default() -> Self {
        Self {
            filter: String::new(),
            sort: TableSort::default(),
            ascending: true,
            revision: [u64::MAX; 3],
            projected_filter: String::new(),
            projected_sort: TableSort::default(),
            projected_ascending: false,
            row_ids: Vec::new(),
            dirty: true,
        }
    }
}

impl TableState {
    fn refresh(&mut self, state: &DocumentState) {
        let revision = [
            state.structure_revision(),
            if self.sort == TableSort::Name || !self.filter.is_empty() {
                state.name_revision()
            } else {
                0
            },
            match self.sort {
                TableSort::X | TableSort::Y => state.position_revision(),
                TableSort::Size => state.size_revision(),
                TableSort::Id | TableSort::Name => 0,
            },
        ];
        if !self.dirty
            && self.revision == revision
            && self.projected_filter == self.filter
            && self.projected_sort == self.sort
            && self.projected_ascending == self.ascending
        {
            return;
        }
        let query = self.filter.to_lowercase();
        let mut objects: Vec<&SceneObject> = state
            .document()
            .objects()
            .iter()
            .filter(|object| {
                query.is_empty()
                    || object.name().to_lowercase().contains(&query)
                    || object.id().0.to_string().contains(&query)
            })
            .collect();
        objects.sort_by(|a, b| {
            let order = match self.sort {
                TableSort::Id => a.id().cmp(&b.id()),
                TableSort::Name => a.name().cmp(b.name()),
                TableSort::X => a.position().x.total_cmp(&b.position().x),
                TableSort::Y => a.position().y.total_cmp(&b.position().y),
                TableSort::Size => a.size().total_cmp(&b.size()),
            };
            if self.ascending {
                order
            } else {
                order.reverse()
            }
        });
        self.row_ids = objects.into_iter().map(SceneObject::id).collect();
        self.revision = revision;
        self.projected_filter.clone_from(&self.filter);
        self.projected_sort = self.sort;
        self.projected_ascending = self.ascending;
        self.dirty = false;
    }
}

pub fn show(ui: &mut Ui, state: &DocumentState, table_state: &mut TableState) -> Vec<Command> {
    let mut commands = Vec::new();
    ui.heading("Data Table");
    ui.horizontal(|ui| {
        ui.label("Filter");
        table_state.dirty |= ui.text_edit_singleline(&mut table_state.filter).changed();
        egui::ComboBox::from_id_salt("table_sort")
            .selected_text(format!("Sort: {:?}", table_state.sort))
            .show_ui(ui, |ui| {
                for sort in [
                    TableSort::Id,
                    TableSort::Name,
                    TableSort::X,
                    TableSort::Y,
                    TableSort::Size,
                ] {
                    table_state.dirty |= ui
                        .selectable_value(&mut table_state.sort, sort, format!("{sort:?}"))
                        .changed();
                }
            });
        table_state.dirty |= ui
            .checkbox(&mut table_state.ascending, "Ascending")
            .changed();
    });
    table_state.refresh(state);
    let clicked = {
        let mut delegate = ObjectTable {
            state,
            rows: &table_state.row_ids,
            clicked: None,
        };
        show_table(
            ui,
            "object_table",
            delegate.rows.len() as u64,
            [
                Column::new(72.0),
                Column::new(180.0),
                Column::new(100.0),
                Column::new(100.0),
                Column::new(84.0),
                Column::new(84.0),
            ]
            .into(),
            &mut delegate,
        );
        delegate.clicked
    };
    if let Some(id) = clicked {
        commands.push(Command::Select([id].into()));
    }
    commands
}

struct ObjectTable<'a> {
    state: &'a DocumentState,
    rows: &'a [ObjectId],
    clicked: Option<ObjectId>,
}

impl TableDelegate for ObjectTable<'_> {
    fn header_cell_ui(&mut self, ui: &mut Ui, cell: &HeaderCellInfo) {
        let title = ["ID", "Name", "X", "Y", "Size", "Visible"]
            .get(cell.col_range.start)
            .copied()
            .unwrap_or("");
        ui.strong(title);
    }

    fn cell_ui(&mut self, ui: &mut Ui, cell: &CellInfo) {
        let Some(id) = self.rows.get(cell.row_nr as usize).copied() else {
            return;
        };
        let Some(object) = self.state.object(id) else {
            return;
        };
        let text = match cell.col_nr {
            0 => object.id().0.to_string(),
            1 => object.name().to_owned(),
            2 => format!("{:.1}", object.position().x),
            3 => format!("{:.1}", object.position().y),
            4 => format!("{:.1}", object.size()),
            5 => if object.visible() { "Yes" } else { "No" }.to_owned(),
            _ => String::new(),
        };
        if ui
            .selectable_label(self.state.selection().contains(&id), text)
            .clicked()
        {
            self.clicked = Some(id);
        }
    }
}
