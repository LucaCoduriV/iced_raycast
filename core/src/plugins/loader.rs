//! Loads compiled plugins (`cdylib`s) at runtime and adapts them to the
//! internal [`Plugin`] trait.
//!
//! Each library is loaded independently via `lib_header_from_path` (which leaks
//! the mapping for the process lifetime, so the plugin's vtable stays valid).
//! `abi_stable` verifies the library's version and type layout before any call,
//! so a plugin built against an incompatible [`plugin_api`] is rejected at load
//! time rather than misbehaving later.

use std::path::{Path, PathBuf};

use abi_stable::{
    library::lib_header_from_path,
    std_types::{RBox, RStr},
};
use directories::ProjectDirs;
use plugin_api::{AbiActionEffect, AbiPluginResult, HostPlugin_TO, PluginModRef};

use super::{ActionEffect, Plugin, PluginAction, PluginResult};
use crate::{APPLICATION, ORGANISATION, QUALIFIER, common::Image};

/// A dynamically-loaded plugin, wrapping its FFI-safe trait object behind the
/// host's native [`Plugin`] interface.
struct DynamicPlugin {
    id: String,
    inner: HostPlugin_TO<'static, RBox<()>>,
}

impl Plugin for DynamicPlugin {
    fn id(&self) -> &str {
        &self.id
    }

    fn query(&self, query: &str) -> Vec<PluginResult> {
        self.inner
            .query(RStr::from(query))
            .into_iter()
            .map(convert_result)
            .collect()
    }
}

/// Directory scanned for installed plugin libraries.
pub fn plugins_dir() -> Option<PathBuf> {
    ProjectDirs::from(QUALIFIER, ORGANISATION, APPLICATION)
        .map(|dirs| dirs.data_local_dir().join("plugins"))
}

/// Load every plugin library found in `dir`. Libraries that fail to load (e.g.
/// an ABI-incompatible build) are logged and skipped rather than aborting.
pub fn load_plugins_from_dir(dir: &Path) -> Vec<Box<dyn Plugin>> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut plugins: Vec<Box<dyn Plugin>> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some(std::env::consts::DLL_EXTENSION) {
            continue;
        }

        match load_plugin(&path) {
            Ok(plugin) => plugins.push(Box::new(plugin)),
            Err(error) => eprintln!("Skipping plugin {}: {error}", path.display()),
        }
    }

    plugins
}

fn load_plugin(path: &Path) -> Result<DynamicPlugin, abi_stable::library::LibraryError> {
    // Loads and leaks the library, then verifies version + layout compatibility.
    let header = lib_header_from_path(path)?;
    let module = header.init_root_module::<PluginModRef>()?;

    let inner = module.new()();
    let id = inner.id().to_string();

    Ok(DynamicPlugin { id, inner })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end load of a real compiled plugin. Opt-in: point
    /// `ICED_RAYCAST_TEST_PLUGIN_DIR` at a directory containing the built
    /// `example_plugin` library.
    #[test]
    fn loads_example_plugin_from_dir() {
        let Ok(dir) = std::env::var("ICED_RAYCAST_TEST_PLUGIN_DIR") else {
            eprintln!("skipped: set ICED_RAYCAST_TEST_PLUGIN_DIR to run");
            return;
        };

        let plugins = load_plugins_from_dir(Path::new(&dir));
        assert!(!plugins.is_empty(), "no plugins loaded from {dir}");

        // The example plugin engages on the "up " keyword and uppercases.
        let results: Vec<_> = plugins.iter().flat_map(|p| p.query("up hello")).collect();
        assert!(
            results.iter().any(|r| r.title == "HELLO"),
            "example plugin produced no matching result"
        );

        // It should stay quiet for unrelated queries.
        let quiet: Vec<_> = plugins.iter().flat_map(|p| p.query("firefox")).collect();
        assert!(quiet.is_empty(), "example plugin fired on an unrelated query");
    }
}

fn convert_result(result: AbiPluginResult) -> PluginResult {
    PluginResult {
        source_id: result.source_id.to_string(),
        section: result.section.to_string(),
        title: result.title.to_string(),
        subtitle: result.subtitle.into_option().map(|s| s.to_string()),
        icon: result
            .icon_path
            .into_option()
            .map(|path| Image::Path(path.to_string())),
        glyph: result.glyph.into_option().and_then(char::from_u32),
        actions: result
            .actions
            .into_iter()
            .map(|action| PluginAction {
                label: action.label.to_string(),
                effect: match action.effect {
                    AbiActionEffect::CopyToClipboard(text) => {
                        ActionEffect::CopyToClipboard(text.to_string())
                    }
                },
            })
            .collect(),
    }
}
