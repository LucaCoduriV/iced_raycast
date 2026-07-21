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
    /// Persisted plugin preference values. A flat list (serialized as a TOML
    /// array of tables) so plugin/preference ids with dots stay simple keys.
    #[serde(default)]
    pub preferences: Vec<StoredPreference>,
}

/// One persisted plugin preference value.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StoredPreference {
    pub plugin: String,
    pub id: String,
    pub value: crate::PreferenceValue,
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

    /// The persisted value of a plugin preference, if the user has set one.
    pub fn preference(&self, plugin: &str, id: &str) -> Option<crate::PreferenceValue> {
        self.preferences
            .iter()
            .find(|pref| pref.plugin == plugin && pref.id == id)
            .map(|pref| pref.value.clone())
    }

    /// Record a plugin preference value, replacing any prior value for it.
    pub fn set_preference(&mut self, plugin: &str, id: &str, value: crate::PreferenceValue) {
        if let Some(existing) = self
            .preferences
            .iter_mut()
            .find(|pref| pref.plugin == plugin && pref.id == id)
        {
            existing.value = value;
        } else {
            self.preferences.push(StoredPreference {
                plugin: plugin.to_string(),
                id: id.to_string(),
                value,
            });
        }
    }

    /// Every persisted preference as `(plugin, id, value)`, for rehydrating
    /// plugins at startup.
    pub fn all_preferences(&self) -> impl Iterator<Item = (&str, &str, crate::PreferenceValue)> {
        self.preferences
            .iter()
            .map(|pref| (pref.plugin.as_str(), pref.id.as_str(), pref.value.clone()))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PreferenceValue;

    #[test]
    fn preferences_persist_and_round_trip_through_toml() {
        let mut state = AppState::default();
        state.set_preference("web.google", "region", PreferenceValue::Choice(2));
        state.set_preference(
            "web.google",
            "open_in_background",
            PreferenceValue::Toggle(true),
        );
        state.set_preference(
            "example.showcase",
            "greeting",
            PreferenceValue::Text("Hi".to_string()),
        );

        // Setting the same (plugin, id) again replaces rather than duplicates.
        state.set_preference("web.google", "region", PreferenceValue::Choice(1));
        assert_eq!(state.preferences.len(), 3);
        assert_eq!(
            state.preference("web.google", "region"),
            Some(PreferenceValue::Choice(1))
        );

        // Serialize to TOML (as `save` does) and read it back.
        let toml = toml::to_string_pretty(&state).unwrap();
        let restored: AppState = toml::from_str(&toml).unwrap();

        // Untagged values survive the round trip with their distinct types.
        assert_eq!(
            restored.preference("web.google", "region"),
            Some(PreferenceValue::Choice(1))
        );
        assert_eq!(
            restored.preference("web.google", "open_in_background"),
            Some(PreferenceValue::Toggle(true))
        );
        assert_eq!(
            restored.preference("example.showcase", "greeting"),
            Some(PreferenceValue::Text("Hi".to_string()))
        );
        assert_eq!(restored.all_preferences().count(), 3);
    }
}
