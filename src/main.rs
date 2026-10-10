mod app;
mod domain;
mod git;
mod settings;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1320.0, 860.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Gitzi",
        options,
        Box::new(|creation_context| Ok(Box::new(app::GitziApp::new(creation_context)))),
    )
}
