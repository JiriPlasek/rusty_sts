use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const APP_NAME: &str = "rusty-sts";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

pub fn enable_start_with_windows() -> Result<(), String> {
    let exe_path =
        std::env::current_exe().map_err(|e| format!("Failed to get exe path: {e}"))?;
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
    let _ = run_key.delete_value(APP_NAME);
    Ok(())
}

pub fn update_registry_path_if_needed() {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = match hkcu.open_subkey(RUN_KEY) {
        Ok(k) => k,
        Err(_) => return,
    };
    let _existing: String = match run_key.get_value(APP_NAME) {
        Ok(v) => v,
        Err(_) => return,
    };
    let _ = enable_start_with_windows();
}
