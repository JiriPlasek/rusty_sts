# Tray Mode, Auto-Sync & Start on Boot — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transform rusty-sts into a background tray app that auto-syncs new `.run` files every 60 seconds with toast notifications.

**Architecture:** Add system tray via `tray-icon` (created before eframe event loop), background polling thread with `Condvar`-based sleep for auto-sync, Windows registry for start-on-boot. The eframe window hides on close instead of exiting. A `--minimized` CLI flag starts the app hidden for boot scenarios.

**Tech Stack:** Rust, eframe/egui 0.31, tray-icon, winrt-notification, winreg

**Spec:** `docs/superpowers/specs/2026-03-24-tray-autosync-design.md`

---

## File Structure

**New files:**
- `assets/icon.png` — 32x32 tray icon (embedded via `include_bytes!`)
- `src/tray.rs` — Tray icon creation, menu setup, event types
- `src/autosync.rs` — Polling thread: checks for new files, requests sync
- `src/notification.rs` — Windows toast notification helper
- `src/startup.rs` — Windows registry start-on-boot toggle

**Modified files:**
- `Cargo.toml` — Add `tray-icon`, `winrt-notification`, `winreg`, `image` dependencies
- `src/main.rs` — Parse `--minimized` flag, create tray icon before eframe, pass to app
- `src/app.rs` — Hide-on-close, tray event handling, auto-sync integration, settings checkboxes
- `src/config.rs` — Add `auto_sync` and `start_with_windows` fields

---

### Task 1: Add Dependencies

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add new crate dependencies**

Add to `Cargo.toml` `[dependencies]`:

```toml
tray-icon = "0.19"
winrt-notification = "0.5"
winreg = "0.56"
image = "0.25"
```

`image` is needed by `tray-icon` to load the PNG into an `Icon`.

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors (just downloading new crates).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add tray-icon, winrt-notification, winreg, image"
```

---

### Task 2: Create Icon Asset

**Files:**
- Create: `assets/icon.png`

- [ ] **Step 1: Create assets directory and a placeholder 32x32 PNG icon**

Create a simple 32x32 PNG. This can be a placeholder (solid color square) or a proper icon. The icon should be visible against both light and dark Windows taskbars.

Store at `assets/icon.png`.

- [ ] **Step 2: Commit**

```bash
git add assets/icon.png
git commit -m "assets: add tray icon"
```

---

### Task 3: Notification Module

**Files:**
- Create: `src/notification.rs`

- [ ] **Step 1: Create `src/notification.rs`**

```rust
use winrt_notification::{Toast, Duration};

pub fn notify_sync_complete(imported: usize) {
    let message = if imported == 1 {
        "1 new run synced".to_string()
    } else {
        format!("{imported} new runs synced")
    };

    // Use POWERSHELL_APP_ID as a reliable fallback — custom app IDs
    // require registration and may fail silently on some Windows versions.
    let _ = Toast::new(Toast::POWERSHELL_APP_ID)
        .title("rusty-sts")
        .text1(&message)
        .duration(Duration::Short)
        .show();
}
```

- [ ] **Step 2: Add `mod notification;` to `src/main.rs`**

Add after the existing mod declarations:

```rust
mod notification;
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles (notification module is unused for now, that's fine).

- [ ] **Step 4: Commit**

```bash
git add src/notification.rs src/main.rs
git commit -m "feat: add notification module for toast notifications"
```

---

### Task 4: Startup (Registry) Module

**Files:**
- Create: `src/startup.rs`

- [ ] **Step 1: Create `src/startup.rs`**

```rust
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const APP_NAME: &str = "rusty-sts";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

pub fn enable_start_with_windows() -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {e}"))?;
    let value = format!("\"{}\" --minimized", exe_path.display());

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run_key, _) = hkcu
        .create_subkey(RUN_KEY)
        .map_err(|e| format!("Failed to open registry key: {e}"))?;
    run_key
        .set_value(APP_NAME, &value)
        .map_err(|e| format!("Failed to write registry value: {e}"))?;
    Ok(())
}

pub fn disable_start_with_windows() -> Result<(), String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(RUN_KEY, winreg::enums::KEY_WRITE)
        .map_err(|e| format!("Failed to open registry key: {e}"))?;
    // Ignore error if value doesn't exist
    let _ = run_key.delete_value(APP_NAME);
    Ok(())
}

/// Re-validate and update the registry path on launch if start_with_windows is enabled.
pub fn update_registry_path_if_needed() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = match hkcu.open_subkey(RUN_KEY) {
        Ok(k) => k,
        Err(_) => return,
    };
    let _existing: String = match run_key.get_value(APP_NAME) {
        Ok(v) => v,
        Err(_) => return, // Not registered, nothing to update
    };
    // Re-write with current exe path
    let _ = enable_start_with_windows();
}
```

- [ ] **Step 2: Add `mod startup;` to `src/main.rs`**

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`

- [ ] **Step 4: Commit**

```bash
git add src/startup.rs src/main.rs
git commit -m "feat: add startup module for Windows registry start-on-boot"
```

---

### Task 5: Config Changes

**Files:**
- Modify: `src/config.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Add new fields to `Config` in `src/config.rs`**

Add a helper function and update the `Config` struct:

```rust
fn default_true() -> bool {
    true
}
```

Update the `Config` struct to:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_token: String,
    pub folder_path: String,
    #[serde(default = "default_true")]
    pub auto_sync: bool,
    #[serde(default)]
    pub start_with_windows: bool,
}
```

- [ ] **Step 2: Update `save_config()` in `src/app.rs`**

The `save_config` method constructs a `Config` manually. Update it to include the new fields. Add `auto_sync` and `start_with_windows` fields to `StsApp`:

In `StsApp` struct, add:
```rust
    auto_sync: bool,
    start_with_windows: bool,
```

In `StsApp::new()`, when loading from config:
```rust
    auto_sync: config.auto_sync,
    start_with_windows: config.start_with_windows,
```

In `StsApp::new()`, in the `None` (no config) branch:
```rust
    auto_sync: true,
    start_with_windows: false,
```

Update `save_config()`:
```rust
    fn save_config(&self) -> Result<(), String> {
        let config = Config {
            api_token: self.api_token.clone(),
            folder_path: self.folder_path.clone(),
            auto_sync: self.auto_sync,
            start_with_windows: self.start_with_windows,
        };
        config.validate()?;
        config.save()
    }
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`

- [ ] **Step 4: Commit**

```bash
git add src/config.rs src/app.rs
git commit -m "feat: add auto_sync and start_with_windows config fields"
```

---

### Task 6: Tray Module

**Files:**
- Create: `src/tray.rs`

- [ ] **Step 1: Create `src/tray.rs`**

```rust
use tray_icon::menu::{Menu, MenuItem, MenuId};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct TrayMenuIds {
    pub open: MenuId,
    pub sync_now: MenuId,
    pub quit: MenuId,
}

pub fn create_tray_icon() -> Result<(TrayIcon, TrayMenuIds), Box<dyn std::error::Error>> {
    let icon_bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(icon_bytes)?.into_rgba8();
    let (width, height) = img.dimensions();
    let icon = Icon::from_rgba(img.into_raw(), width, height)?;

    let menu = Menu::new();
    let open_item = MenuItem::new("Open", true, None);
    let sync_item = MenuItem::new("Sync Now", true, None);
    let quit_item = MenuItem::new("Quit", true, None);

    let ids = TrayMenuIds {
        open: open_item.id().clone(),
        sync_now: sync_item.id().clone(),
        quit: quit_item.id().clone(),
    };

    menu.append(&open_item)?;
    menu.append(&sync_item)?;
    menu.append(&quit_item)?;

    let tray_icon = TrayIconBuilder::new()
        .with_tooltip("rusty-sts")
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()?;

    Ok((tray_icon, ids))
}
```

- [ ] **Step 2: Add `mod tray;` to `src/main.rs`**

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`

- [ ] **Step 4: Commit**

```bash
git add src/tray.rs src/main.rs
git commit -m "feat: add tray module for system tray icon and menu"
```

---

### Task 7: Auto-Sync Polling Module

**Files:**
- Create: `src/autosync.rs`

- [ ] **Step 1: Create `src/autosync.rs`**

```rust
use crate::config::Config;
use crate::detect;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_secs(60);

pub struct ShutdownSignal {
    mutex: Mutex<bool>,
    condvar: Condvar,
}

impl ShutdownSignal {
    pub fn new() -> Self {
        Self {
            mutex: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    /// Wait for the poll interval or until shutdown is signaled.
    /// Returns true if shutdown was requested.
    pub fn wait(&self) -> bool {
        let guard = self.mutex.lock().unwrap();
        let (guard, _timeout) = self
            .condvar
            .wait_timeout(guard, POLL_INTERVAL)
            .unwrap();
        *guard
    }

    /// Signal the polling thread to shut down.
    pub fn shutdown(&self) {
        let mut guard = self.mutex.lock().unwrap();
        *guard = true;
        self.condvar.notify_all();
    }
}

/// Starts the auto-sync polling loop in a background thread.
/// Sends `()` on `sync_request_tx` when new files are detected.
/// The UI thread should call `start_sync()` in response.
/// The `auto_sync_enabled` flag allows the UI to disable polling without stopping the thread.
pub fn start_polling(
    folder_path: String,
    sync_in_progress: Arc<AtomicBool>,
    auto_sync_enabled: Arc<AtomicBool>,
    shutdown: Arc<ShutdownSignal>,
    sync_request_tx: std::sync::mpsc::Sender<()>,
    ctx: egui::Context,
) {
    std::thread::spawn(move || {
        loop {
            if shutdown.wait() {
                break;
            }

            // Skip if auto-sync was disabled by the user
            if !auto_sync_enabled.load(Ordering::Relaxed) {
                continue;
            }

            // Don't request sync if one is already running
            if sync_in_progress.load(Ordering::Relaxed) {
                continue;
            }

            let synced = Config::load_synced_runs();
            let new_count = detect::count_new_run_files(&folder_path, &synced);

            if new_count > 0 {
                let _ = sync_request_tx.send(());
                ctx.request_repaint();
            }
        }
    });
}
```

- [ ] **Step 2: Add `mod autosync;` to `src/main.rs`**

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`

- [ ] **Step 4: Commit**

```bash
git add src/autosync.rs src/main.rs
git commit -m "feat: add autosync polling module"
```

---

### Task 8: Integrate Tray Icon into Main

**Files:**
- Modify: `src/main.rs`
- Modify: `src/app.rs`

This is the big integration task. It wires together the tray icon, auto-sync, hide-on-close, and `--minimized` flag.

- [ ] **Step 1: Update `main.rs` — parse flag, create tray, pass to app**

Replace the contents of `src/main.rs` with:

```rust
#![windows_subsystem = "windows"]

mod app;
mod autosync;
mod config;
mod detect;
mod notification;
mod startup;
mod sync;
mod tray;

fn main() -> eframe::Result {
    let start_minimized = std::env::args().any(|a| a == "--minimized");

    // Load config once — used for visibility check and registry update
    let loaded_config = config::Config::load();
    let has_config = loaded_config.is_some();
    let start_visible = !start_minimized || !has_config;

    // Update registry path if start_with_windows is enabled
    if let Some(cfg) = &loaded_config {
        if cfg.start_with_windows {
            startup::update_registry_path_if_needed();
        }
    }
    drop(loaded_config);

    // Create tray icon before the event loop (required by tray-icon on Windows).
    // _tray_icon must stay alive (not dropped) for the icon to remain visible.
    let (_tray_icon, tray_menu_ids) = tray::create_tray_icon()
        .expect("Failed to create tray icon");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([440.0, 380.0])
            .with_title("rusty-sts")
            .with_visible(start_visible),
        ..Default::default()
    };

    eframe::run_native(
        "rusty-sts",
        options,
        Box::new(move |cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.window_fill = egui::Color32::from_rgb(14, 14, 18);
            visuals.panel_fill = egui::Color32::from_rgb(14, 14, 18);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(24, 24, 30);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(30, 30, 38);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(40, 40, 50);
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(50, 50, 62);
            visuals.selection.bg_fill = egui::Color32::from_rgb(59, 130, 246);
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(app::StsApp::new(
                tray_menu_ids,
                start_visible,
                cc.egui_ctx.clone(),
            )))
        }),
    )
}
```

- [ ] **Step 2: Major update to `src/app.rs`**

This is the largest change. Replace the full contents of `src/app.rs`. Key changes:
- Constructor takes `TrayMenuIds`, `start_visible`, `egui::Context`
- New fields for auto-sync channels, shutdown signal, sync_in_progress flag
- `update()` polls tray menu events and auto-sync requests
- Close interception: hide window instead of exit
- Settings screen gets auto_sync and start_with_windows checkboxes
- Auto-sync polling thread starts after setup

```rust
use crate::autosync::{self, ShutdownSignal};
use crate::config::{Config, API_URL};
use crate::detect;
use crate::notification;
use crate::startup;
use crate::sync::{self, SyncProgress, SyncResult};
use crate::tray::TrayMenuIds;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;

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
    // Config fields
    auto_sync: bool,
    start_with_windows: bool,
    // Ready state
    run_file_count: usize,
    new_run_count: usize,
    last_result: Option<SyncResult>,
    // Syncing state
    progress_rx: Option<mpsc::Receiver<SyncProgress>>,
    result_rx: Option<mpsc::Receiver<SyncResult>>,
    current_progress: Option<SyncProgress>,
    // Tray
    tray_menu_ids: TrayMenuIds,
    window_visible: bool,
    // Auto-sync
    sync_request_rx: Option<mpsc::Receiver<()>>,
    sync_in_progress: Arc<AtomicBool>,
    auto_sync_enabled: Arc<AtomicBool>,
    shutdown_signal: Arc<ShutdownSignal>,
    autosync_started: bool,
    auto_sync_triggered: bool,
    should_quit: bool,
    egui_ctx: egui::Context,
}

impl StsApp {
    pub fn new(
        tray_menu_ids: TrayMenuIds,
        start_visible: bool,
        egui_ctx: egui::Context,
    ) -> Self {
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

        let shutdown_signal = Arc::new(ShutdownSignal::new());
        let sync_in_progress = Arc::new(AtomicBool::new(false));

        match Config::load() {
            Some(config) => {
                let count = detect::count_run_files(&config.folder_path);
                let synced = Config::load_synced_runs();
                let new_count = detect::count_new_run_files(&config.folder_path, &synced);
                let auto_sync_enabled = Arc::new(AtomicBool::new(config.auto_sync));
                Self {
                    state: AppState::Ready,
                    api_token: config.api_token,
                    folder_path: config.folder_path,
                    detected_folders,
                    setup_error: None,
                    auto_sync: config.auto_sync,
                    start_with_windows: config.start_with_windows,
                    run_file_count: count,
                    new_run_count: new_count,
                    last_result: None,
                    progress_rx: None,
                    result_rx: None,
                    current_progress: None,
                    tray_menu_ids,
                    window_visible: start_visible,
                    sync_request_rx: None,
                    sync_in_progress,
                    auto_sync_enabled,
                    shutdown_signal,
                    autosync_started: false,
                    auto_sync_triggered: false,
                    should_quit: false,
                    egui_ctx,
                }
            }
            None => {
                let auto_sync_enabled = Arc::new(AtomicBool::new(true));
                Self {
                    state: AppState::Setup,
                    api_token: String::new(),
                    folder_path: auto_folder,
                    detected_folders,
                    setup_error: None,
                    auto_sync: true,
                    start_with_windows: false,
                    run_file_count: 0,
                    new_run_count: 0,
                    last_result: None,
                    progress_rx: None,
                    result_rx: None,
                    current_progress: None,
                    tray_menu_ids,
                    window_visible: start_visible,
                    sync_request_rx: None,
                    sync_in_progress,
                    auto_sync_enabled,
                    shutdown_signal,
                    autosync_started: false,
                    auto_sync_triggered: false,
                    should_quit: false,
                    egui_ctx,
                }
            },
        }
    }

    fn save_config(&self) -> Result<(), String> {
        let config = Config {
            api_token: self.api_token.clone(),
            folder_path: self.folder_path.clone(),
            auto_sync: self.auto_sync,
            start_with_windows: self.start_with_windows,
        };
        config.validate()?;
        config.save()
    }

    fn start_sync(&mut self) {
        self.sync_in_progress.store(true, Ordering::Relaxed);

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

    fn start_autosync_polling(&mut self) {
        if self.autosync_started {
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.sync_request_rx = Some(rx);

        autosync::start_polling(
            self.folder_path.clone(),
            Arc::clone(&self.sync_in_progress),
            Arc::clone(&self.auto_sync_enabled),
            Arc::clone(&self.shutdown_signal),
            tx,
            self.egui_ctx.clone(),
        );

        self.autosync_started = true;
    }

    fn handle_tray_events(&mut self, ctx: &egui::Context) {
        // Handle menu clicks
        let menu_rx = MenuEvent::receiver();
        while let Ok(event) = menu_rx.try_recv() {
            if event.id == self.tray_menu_ids.open {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.window_visible = true;
            } else if event.id == self.tray_menu_ids.sync_now {
                if self.state == AppState::Ready && self.new_run_count > 0 {
                    self.start_sync();
                }
            } else if event.id == self.tray_menu_ids.quit {
                self.shutdown_signal.shutdown();
                self.should_quit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }

        // Handle tray icon click events (double-click to open window)
        let tray_rx = TrayIconEvent::receiver();
        while let Ok(event) = tray_rx.try_recv() {
            // TrayIconEvent::Click has a button, position, and rect field.
            // On Windows, a double-click fires two Click events in quick succession.
            // For simplicity, treat any left-click on the tray icon as "open window".
            if let TrayIconEvent::Click { button: tray_icon::MouseButton::Left, .. } = event {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                self.window_visible = true;
            }
        }
    }

    fn handle_autosync_requests(&mut self) {
        if let Some(rx) = &self.sync_request_rx {
            if rx.try_recv().is_ok() && self.state == AppState::Ready {
                self.auto_sync_triggered = true;
                self.start_sync();
            }
        }
    }

    fn handle_close_requested(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.should_quit {
                // Allow the close — app is quitting via tray menu
                return;
            }
            // Hide to tray instead of exiting
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.window_visible = false;
        }
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
                egui::TextEdit::singleline(&mut self.folder_path)
                    .desired_width(ui.available_width() - 70.0),
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
                    let synced = Config::load_synced_runs();
                    self.new_run_count =
                        detect::count_new_run_files(&self.folder_path, &synced);
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
        ui.label(format!(
            "{} .run files found ({} new)",
            self.run_file_count, self.new_run_count
        ));

        ui.add_space(12.0);

        ui.horizontal(|ui| {
            let sync_enabled = self.new_run_count > 0;
            if ui
                .add_enabled(
                    sync_enabled,
                    egui::Button::new(if self.new_run_count > 0 {
                        format!("Sync {} new runs", self.new_run_count)
                    } else {
                        "All synced".to_string()
                    }),
                )
                .clicked()
            {
                self.auto_sync_triggered = false;
                self.start_sync();
            }
            if ui.button("Settings").clicked() {
                self.state = AppState::Setup;
            }
        });

        // Settings toggles
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(4.0);

        let prev_auto_sync = self.auto_sync;
        ui.checkbox(&mut self.auto_sync, "Auto-sync new runs");
        if self.auto_sync != prev_auto_sync {
            let _ = self.save_config();
            // Update the shared flag so the polling thread respects the toggle
            self.auto_sync_enabled.store(self.auto_sync, Ordering::Relaxed);
            if self.auto_sync && !self.autosync_started {
                self.start_autosync_polling();
            }
        }

        let prev_start = self.start_with_windows;
        ui.checkbox(&mut self.start_with_windows, "Start with Windows");
        if self.start_with_windows != prev_start {
            let _ = self.save_config();
            if self.start_with_windows {
                let _ = startup::enable_start_with_windows();
            } else {
                let _ = startup::disable_start_with_windows();
            }
        }

        if let Some(result) = &self.last_result {
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Last Sync Result").strong());
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

        if let Some(rx) = &self.progress_rx {
            while let Ok(progress) = rx.try_recv() {
                self.current_progress = Some(progress);
            }
        }

        let mut completed_result = None;
        if let Some(rx) = &self.result_rx {
            if let Ok(result) = rx.try_recv() {
                completed_result = Some(result);
            }
        }

        if let Some(result) = completed_result {
            self.sync_in_progress.store(false, Ordering::Relaxed);

            // Send toast notification if this was an auto-sync
            if self.auto_sync_triggered && result.imported > 0 {
                notification::notify_sync_complete(result.imported);
            }
            self.auto_sync_triggered = false;

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

        ctx.request_repaint();
    }
}

impl eframe::App for StsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Start auto-sync polling if ready and not yet started.
        // The thread checks the auto_sync_enabled flag internally, so start it
        // regardless of the current auto_sync setting — it will respect toggling.
        if self.state != AppState::Setup && !self.autosync_started {
            self.start_autosync_polling();
        }

        self.handle_tray_events(ctx);
        self.handle_autosync_requests();
        self.handle_close_requested(ctx);

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
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check`
Fix any compilation errors.

- [ ] **Step 4: Build and manually test**

Run: `$env:STS_API_URL="https://ststracker.app/"; cargo build --release`

Test manually:
1. Launch the app — window should appear with tray icon
2. Close the window — should minimize to tray, not exit
3. Right-click tray icon — should show Open, Sync Now, Quit
4. Double-click tray icon — should restore window
5. Check "Start with Windows" — verify registry key exists
6. Uncheck — verify registry key removed
7. Wait 60 seconds with auto-sync enabled — verify auto-sync triggers if new files exist
8. Click Quit from tray — app should exit

- [ ] **Step 5: Commit**

```bash
git add src/main.rs src/app.rs
git commit -m "feat: integrate tray icon, auto-sync, hide-on-close, and settings"
```

---

### Task 9: Test `--minimized` Flag

- [ ] **Step 1: Test minimized startup**

Run: `./target/release/rusty-sts.exe --minimized`

Expected:
- No window appears
- Tray icon is visible
- Auto-sync polling is active (if config exists)
- Right-click tray → Open restores the window

- [ ] **Step 2: Test minimized with no config**

Delete config file at `%APPDATA%/rusty-sts/config.json`, then run:
`./target/release/rusty-sts.exe --minimized`

Expected: Window should appear (forced visible) for setup, since there's no config.

- [ ] **Step 3: Final commit if any fixes needed**

```bash
git add -A
git commit -m "fix: address issues found during manual testing"
```
