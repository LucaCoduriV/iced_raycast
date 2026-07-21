//! Example third-party plugin, compiled as a `cdylib` and loaded at runtime.
//!
//! It demonstrates the whole contract: implement [`HostPlugin`], decide when to
//! engage from the query, build results with actions, and export the root
//! module. Trigger it by typing `up <text>` in the launcher — it offers to copy
//! the uppercased text to the clipboard.
//!
//! Build with `cargo build -p example_plugin`, then drop the resulting
//! `libexample_plugin.so` (`.dll` / `.dylib`) into the launcher's plugins
//! directory.

use plugin_api::{
    AbiActionEffect, AbiPluginAction, AbiPluginResult, HostPlugin, export_plugin,
    std_types::{RNone, RSome, RStr, RString, RVec},
};

const KEYWORD: &str = "up ";

#[derive(Default)]
struct UppercasePlugin;

impl HostPlugin for UppercasePlugin {
    fn id(&self) -> RString {
        "example.uppercase".into()
    }

    fn query(&self, query: RStr<'_>) -> RVec<AbiPluginResult> {
        let query = query.as_str();

        // Self-filter: only engage for the "up " keyword prefix.
        let Some(rest) = query.strip_prefix(KEYWORD) else {
            return RVec::new();
        };

        let rest = rest.trim();
        if rest.is_empty() {
            return RVec::new();
        }

        let upper = rest.to_uppercase();

        let result = AbiPluginResult {
            source_id: "example.uppercase".into(),
            section: "Uppercase".into(),
            title: RString::from(upper.as_str()),
            subtitle: RSome(RString::from(rest)),
            icon_path: RNone,
            glyph: RSome(u32::from('A')),
            actions: RVec::from(vec![AbiPluginAction {
                label: "Copy to Clipboard".into(),
                effect: AbiActionEffect::CopyToClipboard(RString::from(upper.as_str())),
            }]),
        };

        RVec::from(vec![result])
    }
}

export_plugin!(UppercasePlugin);
