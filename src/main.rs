mod app;
mod domain;
mod git;
mod settings;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1120.0, 740.0])
            .with_min_inner_size([760.0, 520.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Gitzi",
        options,
        Box::new(|creation_context| Ok(Box::new(app::GitziApp::new(creation_context)))),
    )
}
