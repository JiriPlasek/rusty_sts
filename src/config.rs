use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

pub const API_URL: &str = env!("STS_API_URL");

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_token: String,
    pub folder_path: String,
    #[serde(default = "default_true")]
    pub auto_sync: bool,
    #[serde(default)]
    pub start_with_windows: bool,
}

impl Config {
    fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("rusty-sts"))
    }

    fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.json"))
    }

    fn synced_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("synced_runs.json"))
    }

    pub fn load() -> Option<Config> {
        let path = Self::config_path()?;
        let contents = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("Could not determine config directory")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }
        let json =
            serde_json::to_string_pretty(self).map_err(|e| format!("Failed to serialize: {e}"))?;
        fs::write(&path, json).map_err(|e| format!("Failed to write config: {e}"))?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.api_token.trim().is_empty() {
            return Err("API token is required".to_string());
        }
        if self.folder_path.trim().is_empty() {
            return Err("Folder path is required".to_string());
        }
        let path = PathBuf::from(&self.folder_path);
        if !path.exists() {
            return Err(format!("Folder does not exist: {}", self.folder_path));
        }
        if !path.is_dir() {
            return Err(format!("Path is not a directory: {}", self.folder_path));
        }
        Ok(())
    }

    pub fn load_synced_runs() -> HashSet<String> {
        let path = match Self::synced_path() {
            Some(p) => p,
            None => return HashSet::new(),
        };
        let contents = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return HashSet::new(),
        };
        serde_json::from_str(&contents).unwrap_or_default()
    }

    pub fn save_synced_runs(synced: &HashSet<String>) -> Result<(), String> {
        let path = Self::synced_path().ok_or("Could not determine config directory")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }
        let json = serde_json::to_string(synced).map_err(|e| format!("Failed to serialize: {e}"))?;
        fs::write(&path, json).map_err(|e| format!("Failed to write synced runs: {e}"))?;
        Ok(())
    }
}
