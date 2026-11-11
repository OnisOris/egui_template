use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui::{self, Ui};
use starter_core::{Command, DocumentState, GroupId, TEST_LOADS};
use starter_ui::{PanelId, Theme};

use crate::{
    commands::{self, PREFERENCES_KEY},
    features,
    metrics::MetricsState,
    services::files::{self, FileMessage},
    state::{Preferences, UiState},
    workspace::{self, AppAction},
};

pub struct StarterApp {
    document: DocumentState,
    ui_state: UiState,
    metrics: MetricsState,
    notice: Option<String>,
    file_busy: bool,
    file_sender: Sender<FileMessage>,
    file_receiver: Receiver<FileMessage>,
}

impl StarterApp {
    pub fn new(creation: &eframe::CreationContext<'_>) -> Self {
        let (file_sender, file_receiver) = mpsc::channel();
        let mut app = Self {
            document: DocumentState::default(),
            ui_state: UiState::default(),
            metrics: MetricsState::default(),
            notice: None,
            file_busy: false,
            file_sender,
            file_receiver,
        };
        if let Some(storage) = creation.storage
            && let Some(preferences) = eframe::get_value::<Preferences>(storage, PREFERENCES_KEY)
        {
            app.restore(preferences);
        }
        app
    }

    fn restore(&mut self, preferences: Preferences) {
        let count = preferences.load_size;
        let selection = self.ui_state.restore(preferences);
        if TEST_LOADS.contains(&count) {
            self.document.dispatch(Command::LoadTestScene(count)).ok();
        }
        let selection = selection.into_iter().collect();
        self.document.dispatch(Command::Select(selection)).ok();
    }

    fn action(&mut self, action: AppAction, ctx: &egui::Context) {
        match action {
            AppAction::New => self.run(Command::NewDocument),
            AppAction::Open => self.open_file(ctx),
            AppAction::Save => self.save_file(ctx),
            AppAction::AddObject => self.run(Command::AddObject(GroupId::A)),
            AppAction::DeleteSelection => self.run(Command::DeleteSelection),
            AppAction::ResetView => self.ui_state.viewport.camera.reset(),
            AppAction::ResetLayout => self.ui_state.workspace.reset(),
            AppAction::ToggleTheme => {
                self.ui_state.theme = match self.ui_state.theme {
                    Theme::Dark => Theme::Light,
                    Theme::Light => Theme::Dark,
                }
            }
            AppAction::ResetSettings => {
                let count = self.document.document().objects().len();
                self.ui_state = UiState::default();
                if TEST_LOADS.contains(&count) {
                    self.ui_state.load_size = count;
                }
            }
            AppAction::ToggleRepaint => {
                self.ui_state.continuous_repaint = !self.ui_state.continuous_repaint
            }
            AppAction::SetLoad(count) if TEST_LOADS.contains(&count) => {
                self.ui_state.load_size = count;
                self.run(Command::LoadTestScene(count));
            }
            AppAction::SetLoad(_) => {}
            AppAction::FocusSelection => {
                if let Some(object) = self.document.selected_object() {
                    self.ui_state.viewport.camera.focus(object.position());
                }
            }
        }
    }

    fn run(&mut self, command: Command) {
        commands::dispatch(&mut self.document, &mut self.notice, command);
    }

    fn open_file(&mut self, ctx: &egui::Context) {
        if !self.file_busy {
            self.file_busy = true;
            files::open(self.file_sender.clone(), ctx.clone());
        }
    }

    fn save_file(&mut self, ctx: &egui::Context) {
        if !self.file_busy {
            self.file_busy = true;
            files::save(
                self.document.document().clone(),
                self.file_sender.clone(),
                ctx.clone(),
            );
        }
    }

    fn poll_files(&mut self) {
        while let Ok(message) = self.file_receiver.try_recv() {
            self.file_busy = false;
            match message {
                FileMessage::Opened(None) => self.notice = Some("Open cancelled".to_owned()),
                FileMessage::Opened(Some(Err(error))) | FileMessage::Saved(Some(Err(error))) => {
                    self.notice = Some(error);
                }
                FileMessage::Opened(Some(Ok(document))) => {
                    let count = document.objects().len();
                    if TEST_LOADS.contains(&count) {
                        self.ui_state.load_size = count;
                    }
                    match commands::load_document(&mut self.document, document) {
                        Ok(()) => self.notice = Some("Document opened".to_owned()),
                        Err(error) => self.notice = Some(error.to_string()),
                    }
                }
                FileMessage::Saved(None) => self.notice = Some("Save cancelled".to_owned()),
                FileMessage::Saved(Some(Ok(()))) => self.notice = Some("Document saved".to_owned()),
            }
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if commands::is_text_edit_active(ctx) {
            return;
        }
        let actions = ctx.input(|input| {
            let mut actions = Vec::new();
            if input.modifiers.command && input.key_pressed(egui::Key::N) {
                actions.push(AppAction::New);
            }
            if input.modifiers.command && input.key_pressed(egui::Key::O) {
                actions.push(AppAction::Open);
            }
            if input.modifiers.command && input.key_pressed(egui::Key::S) {
                actions.push(AppAction::Save);
            }
            if input.key_pressed(egui::Key::Delete) {
                actions.push(AppAction::DeleteSelection);
            }
            if input.key_pressed(egui::Key::F) {
                actions.push(AppAction::FocusSelection);
            }
            actions
        });
        for action in actions {
            self.action(action, ctx);
        }
    }

    fn top_ui(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        for action in workspace::menu(ui, &mut self.ui_state.workspace) {
            self.action(action, ctx);
        }
        for action in workspace::toolbar(
            ui,
            self.ui_state.load_size,
            self.ui_state.continuous_repaint,
        ) {
            self.action(action, ctx);
        }
    }
}

impl eframe::App for StarterApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let started = self.metrics.begin();
        let ctx = ui.ctx().clone();
        self.poll_files();
        self.shortcuts(&ctx);
        self.top_ui(ui, &ctx);
        if ui.visuals().dark_mode != matches!(self.ui_state.theme, Theme::Dark) {
            self.ui_state.theme.apply(&ctx);
        }
        let continuous = self.ui_state.continuous_repaint;
        workspace::status(
            ui,
            &self.document,
            &self.metrics,
            self.ui_state.viewport.camera.zoom(),
            continuous,
            self.notice.as_deref(),
        );

        let mut open_gallery_dialog = false;
        let Self {
            document,
            ui_state,
            metrics,
            notice,
            file_sender,
            file_busy,
            ..
        } = self;
        let UiState {
            workspace,
            viewport,
            explorer,
            inspector,
            table,
            analytics,
            gallery,
            ..
        } = ui_state;
        workspace.show(ui, |panel_ui, pane| {
            let commands = match pane {
                PanelId::Map => Vec::new(),
                PanelId::Explorer => starter_ui::show_explorer(panel_ui, document, explorer),
                PanelId::Viewport => features::scene::show(panel_ui, document, viewport),
                PanelId::Inspector => starter_ui::show_inspector(panel_ui, document, inspector),
                PanelId::DataTable => starter_ui::show_data_table(panel_ui, document, table),
                PanelId::Charts => {
                    features::analytics::show(panel_ui, analytics, metrics);
                    Vec::new()
                }
                PanelId::Components => {
                    features::gallery::show(panel_ui, gallery, &mut open_gallery_dialog);
                    Vec::new()
                }
            };
            for command in commands {
                commands::dispatch(document, notice, command);
            }
        });
        if open_gallery_dialog && !*file_busy {
            *file_busy = true;
            files::open(file_sender.clone(), ctx.clone());
        }
        if continuous {
            ctx.request_repaint();
        }
        metrics.finish(started);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let selected: Vec<_> = self.document.selection().iter().copied().collect();
        eframe::set_value(
            storage,
            PREFERENCES_KEY,
            &self.ui_state.preferences(&selected),
        );
    }
}
