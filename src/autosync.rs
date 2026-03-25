use crate::config::{Config, API_URL};
use crate::detect;
use crate::notification;
use crate::sync;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_secs(60);

/// Starts the auto-sync polling loop in a background thread.
/// Performs the sync directly — does not depend on the eframe event loop.
pub fn start_polling(
    folder_path: String,
    api_token: String,
    sync_in_progress: Arc<AtomicBool>,
    auto_sync_enabled: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(POLL_INTERVAL);

            if !auto_sync_enabled.load(Ordering::Relaxed) {
                continue;
            }

            if sync_in_progress.load(Ordering::Relaxed) {
                continue;
            }

            let synced = Config::load_synced_runs();
            let new_count = detect::count_new_run_files(&folder_path, &synced);

            if new_count > 0 {
                sync_in_progress.store(true, Ordering::Relaxed);

                let (progress_tx, _progress_rx) = std::sync::mpsc::channel();
                let result = sync::run_sync(
                    API_URL.to_string(),
                    api_token.clone(),
                    folder_path.clone(),
                    progress_tx,
                );

                if result.imported > 0 {
                    notification::notify_sync_complete(result.imported);
                }

                sync_in_progress.store(false, Ordering::Relaxed);
            }
        }
    });
}
