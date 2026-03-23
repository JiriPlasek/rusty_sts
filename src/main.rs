#![windows_subsystem = "windows"]

mod app;
mod config;
mod detect;
mod sync;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([440.0, 380.0])
            .with_title("rusty-sts"),
        ..Default::default()
    };
    eframe::run_native(
        "rusty-sts",
        options,
        Box::new(|cc| {
            // Dark theme to match the web app
            let mut visuals = egui::Visuals::dark();
            visuals.window_fill = egui::Color32::from_rgb(14, 14, 18);
            visuals.panel_fill = egui::Color32::from_rgb(14, 14, 18);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(24, 24, 30);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(30, 30, 38);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(40, 40, 50);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(50, 50, 62);
            visuals.selection.bg_fill = egui::Color32::from_rgb(59, 130, 246);
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(app::StsApp::new()))
        }),
    )
}
