use eframe::egui::{self, Ui};
use starter_ui::{
    CellInfo, Column, ExplorerTree, HeaderCellInfo, PlotPoint, TableDelegate, line_chart,
    show_table,
};

#[derive(Clone, Copy, PartialEq)]
enum GalleryTab {
    Basics,
    Data,
}

pub struct GalleryState {
    clicks: usize,
    toggle: bool,
    slider: f32,
    text: String,
    choice: String,
    tab: GalleryTab,
    tree: ExplorerTree<u8>,
    signal: Vec<PlotPoint>,
}

impl Default for GalleryState {
    fn default() -> Self {
        Self {
            clicks: 0,
            toggle: true,
            slider: 0.35,
            text: "Edit me".to_owned(),
            choice: "Default".to_owned(),
            tab: GalleryTab::Basics,
            tree: ExplorerTree::default(),
            signal: (0..80)
                .map(|n| PlotPoint::new(n as f64 / 8.0, (n as f64 / 8.0).sin()))
                .collect(),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut GalleryState, open_file: &mut bool) {
    ui.heading("Component Gallery");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.tab, GalleryTab::Basics, "Controls");
        ui.selectable_value(&mut state.tab, GalleryTab::Data, "Data components");
    });
    ui.separator();
    match state.tab {
        GalleryTab::Basics => basics(ui, state, open_file),
        GalleryTab::Data => data_components(ui, state),
    }
}

fn basics(ui: &mut Ui, state: &mut GalleryState, open_file: &mut bool) {
    ui.horizontal_wrapped(|ui| {
        if ui.button("Button").clicked() {
            state.clicks += 1;
        }
        ui.label(format!("Clicked {} times", state.clicks));
        ui.checkbox(&mut state.toggle, "Toggle");
        ui.add(egui::Slider::new(&mut state.slider, 0.0..=1.0).text("Slider"));
        ui.text_edit_singleline(&mut state.text);
        egui::ComboBox::from_id_salt("gallery_combo")
            .selected_text(&state.choice)
            .show_ui(ui, |ui| {
                for choice in ["Default", "Fast", "Accurate"] {
                    ui.selectable_value(&mut state.choice, choice.to_owned(), choice);
                }
            });
        ui.add(egui::ProgressBar::new(state.slider).desired_width(140.0));
        ui.menu_button("Menu", |ui| {
            if ui.button("Open demo file…").clicked() {
                *open_file = true;
                ui.close();
            }
        });
        if ui.button("File dialog…").clicked() {
            *open_file = true;
        }
    });
    ui.label("Tabs are provided by the docking workspace; this page selector is stateful too.");
}

fn data_components(ui: &mut Ui, state: &mut GalleryState) {
    ui.columns(2, |columns| {
        columns[0].strong("Tree view");
        let tree_id = columns[0].id().with("gallery_tree");
        state.tree.show(&mut columns[0], tree_id, |tree| {
            if tree.dir(0_u8, "Project") {
                tree.leaf(1_u8, "Scene");
                tree.leaf(2_u8, "Materials");
            }
            tree.close_dir();
        });
        columns[1].strong("Virtualized table");
        let mut delegate = SampleTable;
        show_table(
            &mut columns[1],
            "gallery_table",
            3,
            [Column::new(70.0), Column::new(120.0), Column::new(90.0)].into(),
            &mut delegate,
        );
    });
    ui.separator();
    line_chart(ui, "gallery_plot", "Sample", state.signal.as_slice().into());
}

struct SampleTable;

impl TableDelegate for SampleTable {
    fn header_cell_ui(&mut self, ui: &mut Ui, cell: &HeaderCellInfo) {
        ui.strong(
            ["ID", "Name", "Value"]
                .get(cell.col_range.start)
                .copied()
                .unwrap_or(""),
        );
    }

    fn cell_ui(&mut self, ui: &mut Ui, cell: &CellInfo) {
        let values = [
            ["01", "Sensor", "12.4"],
            ["02", "Route", "8.7"],
            ["03", "Zone", "3.2"],
        ];
        if let Some(value) = values
            .get(cell.row_nr as usize)
            .and_then(|row| row.get(cell.col_nr))
        {
            ui.label(*value);
        }
    }
}
