mod app;
mod domain;
mod git;
mod settings;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([980.0, 660.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_maximized(false)
            .with_fullscreen(false),
        ..Default::default()
    };

    eframe::run_native(
        "Gitzi",
        options,
        Box::new(|creation_context| Ok(Box::new(app::GitziApp::new(creation_context)))),
    )
}
