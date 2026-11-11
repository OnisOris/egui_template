use bevy_egui::egui::{self, Ui};
use starter_ui::{ExplorerTree, PlotPoint, line_chart};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum GalleryTab {
    #[default]
    Controls,
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
            tab: GalleryTab::Controls,
            tree: ExplorerTree::default(),
            signal: (0..80)
                .map(|n| PlotPoint::new(n as f64 / 8.0, (n as f64 / 8.0).sin()))
                .collect(),
        }
    }
}

pub fn show(ui: &mut Ui, state: &mut GalleryState) {
    ui.heading("Component Gallery");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut state.tab, GalleryTab::Controls, "Controls");
        ui.selectable_value(&mut state.tab, GalleryTab::Data, "Data components");
    });
    ui.separator();
    match state.tab {
        GalleryTab::Controls => controls(ui, state),
        GalleryTab::Data => data(ui, state),
    }
}

fn controls(ui: &mut Ui, state: &mut GalleryState) {
    ui.horizontal_wrapped(|ui| {
        if ui.button("Button").clicked() {
            state.clicks += 1;
        }
        ui.label(format!("Clicked {} times", state.clicks));
        ui.checkbox(&mut state.toggle, "Toggle");
        ui.add(egui::Slider::new(&mut state.slider, 0.0..=1.0).text("Slider"));
        ui.text_edit_singleline(&mut state.text);
        egui::ComboBox::from_id_salt("bevy_gallery_combo")
            .selected_text(&state.choice)
            .show_ui(ui, |ui| {
                for choice in ["Default", "Fast", "Accurate"] {
                    ui.selectable_value(&mut state.choice, choice.to_owned(), choice);
                }
            });
        ui.add(egui::ProgressBar::new(state.slider).desired_width(140.0));
    });
}

fn data(ui: &mut Ui, state: &mut GalleryState) {
    ui.columns(2, |columns| {
        columns[0].strong("Tree view");
        let id = columns[0].id().with("bevy_gallery_tree");
        state.tree.show(&mut columns[0], id, |tree| {
            if tree.dir(0_u8, "Project") {
                tree.leaf(1_u8, "Scene");
                tree.leaf(2_u8, "Materials");
            }
            tree.close_dir();
        });
        columns[1].strong("Plot");
        line_chart(
            &mut columns[1],
            "bevy_gallery_plot",
            "Sample",
            state.signal.as_slice().into(),
        );
    });
}
