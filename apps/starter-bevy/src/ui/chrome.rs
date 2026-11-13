use crate::{state::EditorAction, viewport::ProjectionMode};
use bevy_egui::egui::{self, Ui};
use starter_core::{DocumentState, TEST_LOADS};
use starter_ui::{PanelId, Workspace};

pub fn show(
    ui: &mut Ui,
    workspace: &mut Workspace,
    session: (&DocumentState, &starter_core::ProjectState, bool),
    background: bool,
    projection: ProjectionMode,
    sensitivity: &mut f32,
    notice: Option<&str>,
) -> Vec<EditorAction> {
    let (document, project, benchmark) = session;
    let count = if benchmark {
        document.document().objects().len()
    } else {
        project.document.object_count()
    };
    let mut actions = Vec::new();
    egui::Panel::top("bevy_menu_bar").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                for (label, action) in [
                    ("New Project", EditorAction::New),
                    ("Open Project…", EditorAction::Open),
                    ("Save", EditorAction::Save),
                    ("Save As…", EditorAction::SaveAs),
                    ("Import Asset…", EditorAction::ImportAsset),
                    ("Exit", EditorAction::Exit),
                ] {
                    command(ui, label, action, &mut actions);
                }
            });
            ui.menu_button("Edit", |ui| {
                command(ui, "Add Marker", EditorAction::AddObject, &mut actions);
                command(ui, "Import Model…", EditorAction::ImportAsset, &mut actions);
                command(
                    ui,
                    "Delete Selection",
                    EditorAction::DeleteSelection,
                    &mut actions,
                );
            });
            ui.menu_button("View", |ui| {
                ui.menu_button("Workspace", |ui| {
                    for preset in starter_ui::WorkspacePreset::ALL {
                        if ui
                            .selectable_label(
                                workspace.selected_preset() == Some(preset),
                                preset.title(),
                            )
                            .clicked()
                        {
                            *workspace = Workspace::preset(preset);
                            ui.close();
                        }
                    }
                });
                ui.menu_button("Panels", |ui| {
                    for panel in PanelId::ALL
                        .into_iter()
                        .filter(|p| *p != PanelId::Components)
                    {
                        let mut visible = workspace.is_visible(panel);
                        if ui.checkbox(&mut visible, panel.title()).changed() {
                            workspace.ensure_panel(panel);
                            workspace.set_visible(panel, visible);
                        }
                    }
                });
                command(ui, "Reset Layout", EditorAction::ResetLayout, &mut actions);
                ui.menu_button("Camera", |ui| {
                    command(
                        ui,
                        "Focus Selection",
                        EditorAction::FocusSelection,
                        &mut actions,
                    );
                    command(ui, "Reset View", EditorAction::ResetView, &mut actions);
                    command(
                        ui,
                        &format!("{} → switch projection", projection.label()),
                        EditorAction::ToggleProjection,
                        &mut actions,
                    );
                });
                command(ui, "Toggle Theme", EditorAction::ToggleTheme, &mut actions);
            });
            ui.menu_button("Tools", |ui| {
                ui.menu_button("Demo / Benchmark", |ui| {
                    for count in TEST_LOADS {
                        command(
                            ui,
                            &format!("{count} demo objects"),
                            EditorAction::SetLoad(count),
                            &mut actions,
                        );
                    }
                    command(
                        ui,
                        "Return to Project",
                        EditorAction::ExitDemo,
                        &mut actions,
                    );
                });
                ui.menu_button("Developer", |ui| {
                    command(ui, "Diagnostics", EditorAction::Diagnostics, &mut actions);
                    if ui.button("Component Gallery").clicked() {
                        workspace.ensure_panel(PanelId::Components);
                        workspace.activate(PanelId::Components);
                        ui.close();
                    }
                });
            });
            ui.menu_button("Settings", |ui| {
                command(ui, "Appearance…", EditorAction::Appearance, &mut actions);
                command(ui, "Map Sources…", EditorAction::MapSources, &mut actions);
                ui.menu_button("Camera Controls", |ui| {
                    ui.add(egui::Slider::new(sensitivity, 0.1..=3.0).text("Sensitivity"));
                    ui.label("LMB orbit · RMB pan · wheel zoom");
                });
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if project.dirty {
                    ui.colored_label(starter_ui::style::WARNING, "Unsaved")
                        .on_hover_text("Unsaved changes");
                }
                ui.label(&project.document.name);
            });
        });
    });
    egui::Panel::top("bevy_toolbar").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if benchmark {
                ui.strong("Demo / Benchmark");
                command(
                    ui,
                    "Return to Project",
                    EditorAction::ExitDemo,
                    &mut actions,
                );
            } else {
                ui.menu_button("Add Object", |ui| {
                    command(ui, "Map Marker", EditorAction::AddObject, &mut actions);
                    command(ui, "Import Model…", EditorAction::ImportAsset, &mut actions);
                })
                .response
                .on_hover_text("Add a geographic marker or import a GLB model");
                if ui
                    .selectable_label(
                        project.placing_model.is_none() && !project.placing_marker,
                        "Select",
                    )
                    .on_hover_text("Cancel placement and select objects")
                    .clicked()
                {
                    actions.push(EditorAction::Select);
                }
                command(ui, "Focus", EditorAction::FocusSelection, &mut actions);
            }
            ui.separator();
            for preset in starter_ui::WorkspacePreset::ALL {
                if ui
                    .selectable_label(workspace.selected_preset() == Some(preset), preset.title())
                    .clicked()
                {
                    *workspace = Workspace::preset(preset);
                }
            }
        });
    });
    egui::Panel::bottom("bevy_status_bar").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            if let Some(notice) = notice {
                ui.colored_label(ui.visuals().error_fg_color, notice);
            } else if background {
                ui.spinner();
                ui.label("Loading geographic data…");
            } else {
                ui.colored_label(starter_ui::style::SUCCESS, "Ready");
            }
            ui.separator();
            if let Some(region) = project.document.map_region {
                let (w, h) = region.bounds.dimensions();
                ui.label(format!("Region {:.2} × {:.2} km", w / 1000., h / 1000.));
            } else {
                ui.weak("No region selected");
            }
            ui.separator();
            ui.label(format!("{count} objects"));
            if project.placing_marker || project.placing_model.is_some() {
                ui.label("Click Map to place · Esc cancels");
            }
        });
    });
    actions
}
fn command(ui: &mut Ui, label: &str, action: EditorAction, actions: &mut Vec<EditorAction>) {
    if ui.button(label).clicked() {
        actions.push(action);
        ui.close();
    }
}
