use super::{imagery_view::ImageryView, sources};
use bevy::tasks::AsyncComputeTaskPool;
use bevy_egui::egui::{self, Ui};
pub fn show(ui: &mut Ui, view: &mut ImageryView) {
    if !view.settings_open {
        return;
    }
    let mut open = true;
    let mut apply = false;
    egui::Window::new("Map Sources")
        .open(&mut open)
        .default_width(580.0)
        .max_height(620.0)
        .vscroll(true)
        .show(ui.ctx(), |ui| {
            let settings = view.settings.get_or_insert_with(Default::default);
            ui.horizontal(|ui| {
                if ui.button("Esri World Imagery · Online Preview").clicked() {
                    settings.provider = Some(sources::esri());
                    settings.online_authorized = false;
                }
                if ui.button("Disable Satellite").clicked() {
                    settings.provider = None;
                    settings.online_authorized = false;
                }
            });
            if std::env::var_os("STARTER_IMAGERY_CONFIG").is_some() {
                ui.label("STARTER_IMAGERY_CONFIG overrides saved settings on restart.");
            }
            if let Some(p) = &mut settings.provider {
                ui.label(format!("Selected source: {}", p.name));
                ui.label(if p.allow_permanent_packages {
                    "Storage: RAM and permitted packages"
                } else {
                    "Storage: RAM only"
                });
                ui.label(format!(
                    "Offline Package Export: {}",
                    if p.allow_permanent_packages {
                        "permitted by configuration"
                    } else {
                        "disabled"
                    }
                ));
                ui.text_edit_singleline(&mut p.url);
                ui.label(&p.attribution);
                ui.checkbox(
                    &mut settings.online_authorized,
                    "Online 2D use is authorized for this application",
                );
                ui.label(format!(
                    "Online 2D: {}",
                    if settings.online_authorized {
                        "enabled"
                    } else {
                        "disabled"
                    }
                ));
                ui.checkbox(
                    &mut p.allow_online_3d,
                    "Online 3D use is permitted for this source and application",
                );
                ui.add_enabled_ui(p.allow_permanent_packages, |ui| {
                    ui.checkbox(
                        &mut settings.build_packages,
                        "Build persistent package on Load in 3D",
                    );
                });
                let mut bearer = p.bearer_env.clone().unwrap_or_default();
                ui.horizontal(|ui| {
                    ui.label("Token environment variable (optional)");
                    ui.text_edit_singleline(&mut bearer);
                });
                p.bearer_env = (!bearer.trim().is_empty()).then(|| bearer.trim().to_owned());
                if p.name == "Esri World Imagery" {
                    p.allow_permanent_packages = false;
                    ui.hyperlink_to(
                        "Esri service / attribution information",
                        "https://www.arcgis.com/home/item.html?id=10df2279f9684e4a9f6a7f08febac2a9",
                    );
                }
            } else {
                ui.label("Satellite source not configured");
            }
            let status = view
                .tiles
                .as_ref()
                .map(|t| {
                    t.error.as_deref().unwrap_or(if t.ready() {
                        "Connected · tiles received"
                    } else {
                        "Configured · waiting for a visible tile request"
                    })
                })
                .or(view.message.as_deref())
                .unwrap_or("Checking source…");
            ui.label(status);
            ui.separator();
            ui.label("Existing Map Package Directory");
            let directory = settings.package_directory.get_or_insert_with(|| {
                super::package_browser::default_directory()
                    .to_string_lossy()
                    .into_owned()
            });
            ui.text_edit_singleline(directory);
            if ui
                .add_enabled(view.task.is_none(), egui::Button::new("Apply and Save"))
                .clicked()
            {
                apply = true;
            }
        });
    view.settings_open = open;
    if apply && let Some(settings) = view.settings.clone() {
        view.tiles = None;
        view.service = None;
        view.enabled = false;
        view.message = None;
        view.task = Some(AsyncComputeTaskPool::get().spawn(async move {
            sources::save(&settings)?;
            Ok(settings)
        }));
    }
}
