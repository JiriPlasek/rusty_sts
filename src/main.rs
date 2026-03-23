mod app;
mod config;
mod detect;
mod sync;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([420.0, 350.0])
            .with_title("rusty-sts"),
        ..Default::default()
    };
    eframe::run_native(
        "rusty-sts",
        options,
        Box::new(|_cc| Ok(Box::new(app::StsApp::new()))),
    )
}
