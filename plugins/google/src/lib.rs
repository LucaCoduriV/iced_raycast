//! Google search plugin. Lists a "Search Google" command; activating it prompts
//! for a query and opens the results in your default browser.

use plugin_api::{
    export_plugin,
    std_types::{RNone, ROption, RSome, RStr, RString, RVec},
    AbiActionEffect, AbiCommand, HostPlugin,
};

const PLUGIN_ID: &str = "web.google";

#[derive(Default)]
struct GooglePlugin;

impl HostPlugin for GooglePlugin {
    fn id(&self) -> RString {
        PLUGIN_ID.into()
    }

    fn commands(&self) -> RVec<AbiCommand> {
        RVec::from(vec![AbiCommand {
            id: "search".into(),
            title: "Search Google".into(),
            subtitle: RSome("Open a web search in your browser".into()),
            keywords: RVec::from(vec!["google".into(), "search".into(), "web".into()]),
            icon_path: RNone,
            glyph: RSome(u32::from('G')),
            category: "Web Search".into(),
            needs_argument: true,
            argument_placeholder: RSome("Search query…".into()),
        }])
    }

    fn run_command(&self, command_id: RStr<'_>, argument: ROption<RString>) -> AbiActionEffect {
        if command_id.as_str() != "search" {
            return AbiActionEffect::None;
        }

        let term = argument
            .into_option()
            .map(|s| s.as_str().trim().to_string())
            .unwrap_or_default();
        if term.is_empty() {
            return AbiActionEffect::None;
        }

        let url = format!("https://www.google.com/search?q={}", encode(&term));
        AbiActionEffect::OpenUrl(RString::from(url))
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
