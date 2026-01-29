use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone)]
pub enum Image {
    Data(Vec<u8>),
    Path(String),
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct AppState {
    pub usage_stats: HashMap<String, UsageInfo>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UsageInfo {
    pub count: u32,
    pub last_used: u64,
}

use std::time::{SystemTime, UNIX_EPOCH};

impl AppState {
    fn get_path() -> std::path::PathBuf {
        let proj_dirs = ProjectDirs::from("com", "your_name", "your_launcher")
            .expect("Could not find config directory");
        let data_dir = proj_dirs.data_local_dir();

        // Ensure the directory exists
        fs::create_dir_all(data_dir).ok();
        data_dir.join("state.json")
    }

    pub fn record_usage(&mut self, entity: &super::Entity) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let stats = self
            .usage_stats
            .entry(entity.name().to_string())
            .or_insert(UsageInfo {
                count: 0,
                last_used: now,
            });

        stats.count += 1;
        stats.last_used = now;
    }

    pub fn load() -> Self {
        let path = Self::get_path();
        fs::read_to_string(path)
            .map(|content| toml::from_str(&content).expect("Couldn't read settings"))
            .unwrap_or_default()
    }

    pub fn get_score(&self, entity: &super::Entity) -> u32 {
        self.usage_stats
            .get(entity.name())
            .map_or(0, |info| info.count)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_path();
        let json = toml::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}
