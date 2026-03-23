use crate::config::{Config, API_URL};
use crate::detect;
use crate::sync::{self, SyncProgress, SyncResult};
use std::sync::mpsc;

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    Setup,
    Ready,
    Syncing,
}

pub struct StsApp {
    state: AppState,
    // Setup fields
    api_token: String,
    folder_path: String,
    detected_folders: Vec<String>,
    setup_error: Option<String>,
    // Ready state
    run_file_count: usize,
    new_run_count: usize,
    last_result: Option<SyncResult>,
    // Syncing state
    progress_rx: Option<mpsc::Receiver<SyncProgress>>,
    result_rx: Option<mpsc::Receiver<SyncResult>>,
    current_progress: Option<SyncProgress>,
}

impl StsApp {
    pub fn new() -> Self {
        let detected = detect::detect_save_folders();
        let detected_folders: Vec<String> = detected
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let auto_folder = if detected_folders.len() == 1 {
            detected_folders[0].clone()
        } else {
            String::new()
        };

        match Config::load() {
            Some(config) => {
                let count = detect::count_run_files(&config.folder_path);
                let synced = Config::load_synced_runs();
                let new_count = detect::count_new_run_files(&config.folder_path, &synced);
                Self {
                    state: AppState::Ready,
                    api_token: config.api_token,
                    folder_path: config.folder_path,
                    detected_folders,
                    setup_error: None,
                    run_file_count: count,
                    new_run_count: new_count,
                    last_result: None,
                    progress_rx: None,
                    result_rx: None,
                    current_progress: None,
                }
            }
            None => Self {
                state: AppState::Setup,
                api_token: String::new(),
                folder_path: auto_folder,
                detected_folders,
                setup_error: None,
                run_file_count: 0,
                new_run_count: 0,
                last_result: None,
                progress_rx: None,
                result_rx: None,
                current_progress: None,
            },
        }
    }

    fn save_config(&self) -> Result<(), String> {
        let config = Config {
            api_token: self.api_token.clone(),
            folder_path: self.folder_path.clone(),
        };
        config.validate()?;
        config.save()
    }

    fn start_sync(&mut self) {
        let (progress_tx, progress_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();

        let api_url = API_URL.to_string();
        let api_token = self.api_token.clone();
        let folder_path = self.folder_path.clone();

        std::thread::spawn(move || {
            let result = sync::run_sync(api_url, api_token, folder_path, progress_tx);
            let _ = result_tx.send(result);
        });

        self.progress_rx = Some(progress_rx);
        self.result_rx = Some(result_rx);
        self.current_progress = None;
        self.state = AppState::Syncing;
    }

    fn render_setup(&mut self, ui: &mut egui::Ui) {
        ui.heading("Setup");
        ui.add_space(8.0);

        ui.label("API Token (from Settings page on the website):");
        ui.add(egui::TextEdit::singleline(&mut self.api_token).desired_width(f32::INFINITY));
        ui.add_space(8.0);

        ui.label("Save Folder:");
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.folder_path).desired_width(ui.available_width() - 70.0),
            );
            if ui.button("Browse").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.folder_path = path.to_string_lossy().to_string();
                }
            }
        });

        if !self.detected_folders.is_empty() {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Detected folders:")
                    .small()
                    .color(egui::Color32::GRAY),
            );
            let folders = self.detected_folders.clone();
            for folder in &folders {
                if ui
                    .small_button(truncate_path(folder, 60))
                    .on_hover_text(folder)
                    .clicked()
                {
                    self.folder_path = folder.clone();
                }
            }
        }

        ui.add_space(12.0);

        if let Some(err) = &self.setup_error {
            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err.as_str());
            ui.add_space(4.0);
        }

        if ui.button("Save & Continue").clicked() {
            match self.save_config() {
                Ok(()) => {
                    self.setup_error = None;
                    self.run_file_count = detect::count_run_files(&self.folder_path);
                    self.state = AppState::Ready;
                }
                Err(e) => {
                    self.setup_error = Some(e);
                }
            }
        }
    }

    fn render_ready(&mut self, ui: &mut egui::Ui) {
        ui.heading("Ready to Sync");
        ui.add_space(8.0);

        ui.label(format!("Folder: {}", self.folder_path));
        self.run_file_count = detect::count_run_files(&self.folder_path);
        let synced = Config::load_synced_runs();
        self.new_run_count = detect::count_new_run_files(&self.folder_path, &synced);
        ui.label(format!("{} .run files found ({} new)", self.run_file_count, self.new_run_count));

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            let sync_enabled = self.new_run_count > 0;
            if ui
                .add_enabled(sync_enabled, egui::Button::new(if self.new_run_count > 0 {
                    format!("Sync {} new runs", self.new_run_count)
                } else {
                    "All synced".to_string()
                }))
                .clicked()
            {
                self.start_sync();
            }
            if ui.button("Settings").clicked() {
                self.state = AppState::Setup;
            }
        });

        if let Some(result) = &self.last_result {
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Last Sync Result")
                    .strong(),
            );
            ui.label(format!("Imported: {}", result.imported));
            ui.label(format!("Skipped: {}", result.skipped));
            if !result.errors.is_empty() {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    format!("Errors: {}", result.errors.len()),
                );
                for err in &result.errors {
                    ui.label(
                        egui::RichText::new(format!("  - {err}"))
                            .small()
                            .color(egui::Color32::from_rgb(255, 150, 150)),
                    );
                }
            }
        }
    }

    fn render_syncing(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.heading("Syncing...");
        ui.add_space(8.0);

        // Check for progress updates
        if let Some(rx) = &self.progress_rx {
            while let Ok(progress) = rx.try_recv() {
                self.current_progress = Some(progress);
            }
        }

        // Check for completion
        let mut completed_result = None;
        if let Some(rx) = &self.result_rx {
            if let Ok(result) = rx.try_recv() {
                completed_result = Some(result);
            }
        }

        if let Some(result) = completed_result {
            self.last_result = Some(result);
            self.progress_rx = None;
            self.result_rx = None;
            self.current_progress = None;
            self.state = AppState::Ready;
            return;
        }

        if let Some(progress) = &self.current_progress {
            ui.label(&progress.phase);
            if progress.total > 0 {
                let fraction = progress.current as f32 / progress.total as f32;
                ui.add(
                    egui::ProgressBar::new(fraction)
                        .text(format!("{}/{} files", progress.current, progress.total)),
                );
            }
        } else {
            ui.label("Starting...");
            ui.spinner();
        }

        // Keep repainting while syncing
        ctx.request_repaint();
    }
}

impl eframe::App for StsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.state.clone() {
                AppState::Setup => self.render_setup(ui),
                AppState::Ready => self.render_ready(ui),
                AppState::Syncing => self.render_syncing(ctx, ui),
            }
        });
    }
}

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}
