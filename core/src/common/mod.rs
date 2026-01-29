use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone)]
pub enum Image {
    Data(Vec<u8>),
    Path(String),
}

#[derive(Serialize, Deserialize, Default)]
pub struct AppState {
    pub usage_stats: HashMap<String, UsageInfo>,
}

#[derive(Serialize, Deserialize)]
pub struct UsageInfo {
    pub count: u32,
    pub last_used: u64,
}

impl AppState {
    fn get_path() -> std::path::PathBuf {
        let proj_dirs = ProjectDirs::from("com", "your_name", "your_launcher")
            .expect("Could not find config directory");
        let data_dir = proj_dirs.data_local_dir();

        // Ensure the directory exists
        fs::create_dir_all(data_dir).ok();
        data_dir.join("state.json")
    }

    pub fn load() -> Self {
        let path = Self::get_path();
        fs::read_to_string(path)
            .map(|content| toml::from_str(&content).expect("Couldn't read settings"))
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_path();
        let json = toml::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
