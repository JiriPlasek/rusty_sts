use winrt_notification::{Duration, Toast};

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
