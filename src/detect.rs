use std::path::PathBuf;

/// Scan known locations for STS2 save history folders.
/// On Windows: %APPDATA%\SlayTheSpire2\steam\<steam_id>\profile1\saves\history\
pub fn detect_save_folders() -> Vec<PathBuf> {
    let mut results = Vec::new();

    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let base = PathBuf::from(appdata).join("SlayTheSpire2").join("steam");
            if base.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&base) {
                    for entry in entries.flatten() {
                        let history = entry
                            .path()
                            .join("profile1")
                            .join("saves")
                            .join("history");
                        if history.is_dir() {
                            results.push(history);
                        }
                    }
                }
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        // On macOS/Linux, check common Steam paths
        if let Some(home) = dirs::home_dir() {
            let candidates = vec![
                home.join(".local/share/Steam/steamapps/compatdata/2868840/pfx/drive_c/users/steamuser/AppData/Roaming/SlayTheSpire2/steam"),
                home.join("Library/Application Support/SlayTheSpire2/steam"),
            ];
            for base in candidates {
                if base.is_dir() {
                    if let Ok(entries) = std::fs::read_dir(&base) {
                        for entry in entries.flatten() {
                            let history = entry
                                .path()
                                .join("profile1")
                                .join("saves")
                                .join("history");
                            if history.is_dir() {
                                results.push(history);
                            }
                        }
                    }
                }
            }
        }
    }

    results
}

/// Count .run files in a directory.
pub fn count_run_files(folder: &str) -> usize {
    let path = PathBuf::from(folder);
    if !path.is_dir() {
        return 0;
    }
    std::fs::read_dir(&path)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.path()
                        .extension()
                        .is_some_and(|ext| ext == "run")
                })
                .count()
        })
        .unwrap_or(0)
}
