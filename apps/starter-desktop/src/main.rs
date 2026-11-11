mod app;
mod commands;
mod features;
mod metrics;
mod services;
mod state;
mod workspace;

#[cfg(test)]
mod chain;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Rust Desktop UI Starter",
        options,
        Box::new(|creation| Ok(Box::new(app::StarterApp::new(creation)))),
    )
}
