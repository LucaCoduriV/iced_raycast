use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Clone)]
pub enum Image {
    Bytes(Vec<u8>),
    Path(String),
    Rgba(u32, u32, Vec<u8>),
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct AppState {
    pub usage_stats: HashMap<String, UsageInfo>,
    /// Most-recent-first list of arguments previously passed to each entity,
    /// keyed by entity name. `#[serde(default)]` keeps older state files valid.
    #[serde(default)]
    pub recent_arguments: HashMap<String, Vec<String>>,
}

/// Maximum number of recent arguments retained per entity.
const MAX_RECENT_ARGUMENTS: usize = 8;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UsageInfo {
    pub count: u32,
    pub last_used: u64,
}

use std::time::{SystemTime, UNIX_EPOCH};

use crate::{APPLICATION, ORGANISATION, QUALIFIER};

impl AppState {
    fn get_path() -> std::path::PathBuf {
        let proj_dirs = ProjectDirs::from(QUALIFIER, ORGANISATION, APPLICATION)
            .expect("Could not find config directory");
        let data_dir = proj_dirs.data_local_dir();

        // Ensure the directory exists
        fs::create_dir_all(data_dir).ok();
        data_dir.join("state.toml")
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
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Recent arguments previously used for `entity_name`, most recent first.
    pub fn recent_arguments(&self, entity_name: &str) -> Vec<String> {
        self.recent_arguments
            .get(entity_name)
            .cloned()
            .unwrap_or_default()
    }

    /// Record `argument` as the most recent argument used for `entity_name`,
    /// de-duplicating and capping the retained history.
    pub fn record_argument(&mut self, entity_name: &str, argument: &str) {
        if argument.is_empty() {
            return;
        }

        let history = self
            .recent_arguments
            .entry(entity_name.to_string())
            .or_default();

        history.retain(|existing| existing != argument);
        history.insert(0, argument.to_string());
        history.truncate(MAX_RECENT_ARGUMENTS);
    }

    pub fn get_score(&self, entity: &super::Entity) -> u32 {
        self.usage_stats
            .get(entity.name())
            .map_or(0, |info| info.count)
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::get_path();
        let contents = toml::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}
