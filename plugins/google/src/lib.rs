//! Google search plugin. Type `g <query>` and press Enter to open the results
//! in your default browser.

use plugin_api::{
    export_plugin,
    std_types::{RNone, RSome, RStr, RString, RVec},
    AbiActionEffect, AbiPluginAction, AbiPluginResult, HostPlugin,
};

const PLUGIN_ID: &str = "web.google";

#[derive(Default)]
struct GooglePlugin;

impl HostPlugin for GooglePlugin {
    fn id(&self) -> RString {
        PLUGIN_ID.into()
    }

    fn query(&self, query: RStr<'_>) -> RVec<AbiPluginResult> {
        let Some(term) = query
            .as_str()
            .strip_prefix("g ")
            .map(str::trim)
            .filter(|term| !term.is_empty())
        else {
            return RVec::new();
        };

        let url = format!("https://www.google.com/search?q={}", encode(term));

        RVec::from(vec![AbiPluginResult {
            source_id: PLUGIN_ID.into(),
            section: "Web Search".into(),
            title: RString::from(format!("Search Google for “{term}”")),
            subtitle: RSome("Open results in your browser".into()),
            icon_path: RNone,
            glyph: RSome(u32::from('G')),
            actions: RVec::from(vec![AbiPluginAction {
                label: "Open in Browser".into(),
                effect: AbiActionEffect::OpenUrl(RString::from(url)),
            }]),
        }])
    }
}

/// Minimal `application/x-www-form-urlencoded` encoding for a query string.
fn encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

export_plugin!(GooglePlugin);
