# Tray Mode, Auto-Sync & Start on Boot

## Summary

Transform rusty-sts from a manual sync tool into a background service that lives in the Windows system tray, automatically uploads new `.run` files on a polling interval, and optionally starts with Windows.

## Goals

- App minimizes to system tray instead of closing
- New `.run` files are automatically detected and uploaded without user intervention
- Windows toast notifications inform the user when runs are synced
- Optional "Start with Windows" launches the app minimized to tray on boot
- First-time setup experience remains unchanged (window opens, user configures token + folder)

## Architecture

### System Tray (`tray-icon` crate)

On startup, create a system tray icon with a right-click context menu:
- **Open** — show the eframe window
- **Sync Now** — trigger an immediate sync
- **Quit** — exit the app

Behavior:
- Clicking the window's X button **hides** the window instead of exiting. The app continues running in the tray.
- Double-clicking the tray icon opens the window.
- Tray icon uses an embedded 32x32 PNG via `include_bytes!()`, stored at `assets/icon.png`.
- Tray tooltip shows status: "rusty-sts — X new runs".

**Event loop integration**: The `TrayIcon` must be created in `main()` **before** `eframe::run_native()`, since it needs the Windows message loop that winit provides. The `MenuEvent::receiver()` channel is passed into `StsApp` via a modified constructor (`StsApp::new(menu_event_rx, start_minimized)`). Menu events are polled in the `update()` loop via `try_recv()`.

**Window hide/show**: Override `fn on_close_event(&mut self) -> bool` returning `false` to prevent exit. Use `ctx.send_viewport_cmd(ViewportCommand::Visible(false))` to hide the window. The tray "Open" action uses `ViewportCommand::Visible(true)` + `ViewportCommand::Focus` to restore it.

### Auto-Sync via Polling

A background thread runs after setup is complete:
1. Wait 60 seconds (via `Condvar` with timeout — see Graceful Shutdown below)
2. Check save folder for new `.run` files (reuses `count_new_run_files` logic)
3. If new files exist, send a "sync requested" message to the UI thread
4. The UI thread receives this and calls `start_sync()` (same path as the manual Sync button), ensuring the state machine stays consistent and preventing double-sync
5. The UI thread checks `AppState::Syncing` before honoring the request — if already syncing, the request is ignored
6. On sync completion, if it was triggered by auto-sync, send a Windows toast notification: "rusty-sts: Synced X new runs"
7. Loop back to step 1

**Event loop wakeup**: The polling thread holds a clone of `egui::Context` (which is `Send + Sync`) and calls `ctx.request_repaint()` after sending the sync-requested message. This wakes the eframe event loop even when the window is hidden.

**Concurrency**: An `AtomicBool` (`sync_in_progress`) is shared between UI and polling thread. The polling thread checks it before sending a sync request, and the UI thread sets/clears it in `start_sync()` / sync completion. This is a belt-and-suspenders guard alongside the state check.

The polling interval is fixed at 60 seconds. Not configurable to keep settings simple.

### Start on Boot

A "Start with Windows" checkbox in the Settings screen. When toggled:
- **Enabled**: Writes a registry key at `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` with value `"<current_exe_path>" --minimized` (quoted path for spaces)
- **Disabled**: Removes that registry key

Uses `std::env::current_exe()` for the path. **Limitation**: If the user moves the executable after enabling, the registry entry becomes stale. On each app launch, if `start_with_windows` is true in config, re-validate and update the registry path to match the current exe location.

### `--minimized` CLI Flag

Checked in `main.rs` via `std::env::args()`. When present:
- The eframe window starts **hidden** via `ViewportBuilder::with_visible(false)`
- App goes straight to tray with auto-sync active
- Used by the registry startup entry so the app launches silently on boot

**Edge case**: If `--minimized` is passed but no valid config exists (deleted config, first run), force the window to be visible so the user can complete setup. Check `Config::load()` before applying the minimized flag.

Normal launch (double-click exe) opens the window as usual.

### Config Changes

Add two new fields to `Config` in `config.rs`:

```rust
#[serde(default = "default_true")]
pub auto_sync: bool,         // default: true

#[serde(default)]
pub start_with_windows: bool, // default: false
```

Both use `#[serde(default)]` so existing config files deserialize without issues. Persisted in the existing `config.json`. Note: all places that construct `Config` (e.g., `save_config()` in `app.rs`) must be updated to include the new fields.

The Settings screen gains two checkboxes:
- "Auto-sync new runs" — controls whether the polling thread uploads automatically
- "Start with Windows" — toggles the registry entry immediately on change

### Graceful Shutdown

The polling thread uses `Condvar` with a 60-second timeout instead of `thread::sleep`. When the user clicks "Quit" from the tray menu, the main thread signals the condvar, waking the polling thread immediately for clean exit. This avoids up to 60 seconds of delay on quit.

### New Dependencies

| Crate | Purpose |
|-------|---------|
| `tray-icon` | System tray icon and context menu |
| `winrt-notification` | Windows toast notifications |
| `winreg` | Windows registry access (start-on-boot) |

All three are Windows-specific. Gate behind `#[cfg(target_os = "windows")]` for compilation compatibility, though the app primarily targets Windows.

## New Modules

- `src/tray.rs` — Tray icon creation, menu setup, event handling
- `src/autosync.rs` — Polling thread that checks for new files and requests sync
- `src/notification.rs` — Toast notification helper

## State Changes

`AppState` remains the same (Setup, Ready, Syncing). The auto-sync polling thread sends a sync request to the UI, which calls `start_sync()` — same code path as the manual Sync button. The tray menu's "Sync Now" also reuses `start_sync()`.

New app fields:
- `menu_event_rx` — receiver for tray menu click events (from `tray-icon`)
- `autosync_request_rx` — receiver for "please sync" messages from the polling thread
- `sync_in_progress: Arc<AtomicBool>` — shared flag for concurrency guard
- `shutdown_signal` — `Condvar` + `Mutex<bool>` for clean polling thread shutdown
- `window_visible: bool` — tracks whether the eframe window is shown or hidden
- `egui_ctx: Option<egui::Context>` — stored after first `update()` call, cloned to polling thread

## Flow

1. User downloads and runs `rusty-sts.exe` for the first time
2. Window opens, user enters API token and selects save folder
3. After setup, auto-sync polling starts immediately
4. User can close the window — app minimizes to tray
5. Every 60 seconds, the app checks for new `.run` files and uploads them
6. Toast notifications appear when runs are synced
7. If user enables "Start with Windows", the app launches minimized on next boot
8. Tray right-click menu provides Open, Sync Now, and Quit options

## Icon Asset

A 32x32 PNG icon stored at `assets/icon.png`, embedded via `include_bytes!()`. Used for both the tray icon and the toast notification icon. Needs to be created or sourced before implementation.
