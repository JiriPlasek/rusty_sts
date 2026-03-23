use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const API_URL: &str = env!("STS_API_URL");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub api_token: String,
    pub folder_path: String,
}

impl Config {
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("rusty-sts").join("config.json"))
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
}
