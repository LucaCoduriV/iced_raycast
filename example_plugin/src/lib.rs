//! Example third-party plugin, compiled as a `cdylib` and loaded at runtime.
//!
//! It showcases the whole contract: producing list results, and pushing each of
//! the three interactive view layouts. Triggers (type in the launcher):
//!
//! - `up <text>`  — copy the uppercased text (list result, copy effect)
//! - `grid`       — open a searchable **grid** view
//! - `detail`     — open a **detail** view
//! - `form`       — open a **form** view
//!
//! Build with `cargo build -p example_plugin`, then drop the resulting
//! `libexample_plugin.so` (`.dll` / `.dylib`) into the launcher's plugins dir.

use plugin_api::{
    export_plugin,
    std_types::{RNone, RSome, RStr, RString, RVec},
    AbiActionEffect, AbiFieldKind, AbiFieldValueKind, AbiFormField, AbiGridItem, AbiImageSource,
    AbiKeyValue, AbiPluginAction, AbiPluginResult, AbiView, AbiViewBody, AbiViewEvent,
    AbiViewEventKind, AbiViewResponse, HostPlugin,
};

const PLUGIN_ID: &str = "example.showcase";

#[derive(Default)]
struct ShowcasePlugin;

impl HostPlugin for ShowcasePlugin {
    fn id(&self) -> RString {
        PLUGIN_ID.into()
    }

    fn query(&self, query: RStr<'_>) -> RVec<AbiPluginResult> {
        let query = query.as_str();
        let mut results: Vec<AbiPluginResult> = Vec::new();

        if let Some(rest) = query.strip_prefix("up ") {
            let rest = rest.trim();
            if !rest.is_empty() {
                results.push(uppercase_result(rest));
            }
        }

        if query == "grid" {
            results.push(open_result(
                "Grid Demo",
                "Open a searchable grid view",
                'G',
                AbiActionEffect::PushView(grid_view("")),
            ));
        }

        if query == "detail" {
            results.push(open_result(
                "Detail Demo",
                "Open a detail view",
                'D',
                AbiActionEffect::PushView(detail_view()),
            ));
        }

        if query == "form" || query == "snippet" {
            results.push(open_result(
                "Create Snippet",
                "Open a form view",
                'F',
                AbiActionEffect::PushView(form_view()),
            ));
        }

        results.into()
    }

    fn handle_event(&self, event: AbiViewEvent) -> AbiViewResponse {
        match (event.view_id.as_str(), event.kind) {
            // Grid: typing re-queries; activating a cell copies its label.
            ("grid-demo", AbiViewEventKind::Search(term)) => {
                AbiViewResponse::Update(grid_view(term.as_str()))
            }
            ("grid-demo", AbiViewEventKind::Activate(id)) => {
                AbiViewResponse::Effect(AbiActionEffect::CopyToClipboard(id))
            }
            // Form: submit copies the collected values.
            ("snippet-form", AbiViewEventKind::Submit(values)) => {
                let text = values
                    .iter()
                    .map(|value| {
                        let rendered = match &value.value {
                            AbiFieldValueKind::Text(text) => text.to_string(),
                            AbiFieldValueKind::Toggle(on) => on.to_string(),
                            AbiFieldValueKind::Choice(index) => index.to_string(),
                        };
                        format!("{} = {}", value.id, rendered)
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                AbiViewResponse::Effect(AbiActionEffect::CopyToClipboard(RString::from(text)))
            }
            // Detail: primary action just closes the launcher.
            ("detail-demo", AbiViewEventKind::Submit(_)) => {
                AbiViewResponse::Effect(AbiActionEffect::Close)
            }
            _ => AbiViewResponse::None,
        }
    }
}

fn uppercase_result(text: &str) -> AbiPluginResult {
    let upper = text.to_uppercase();
    AbiPluginResult {
        source_id: PLUGIN_ID.into(),
        section: "Uppercase".into(),
        title: RString::from(upper.as_str()),
        subtitle: RSome(RString::from(text)),
        icon_path: RNone,
        glyph: RSome(u32::from('A')),
        actions: RVec::from(vec![AbiPluginAction {
            label: "Copy to Clipboard".into(),
            effect: AbiActionEffect::CopyToClipboard(RString::from(upper.as_str())),
        }]),
    }
}

/// A list result whose default action pushes a view.
fn open_result(
    title: &str,
    subtitle: &str,
    glyph: char,
    effect: AbiActionEffect,
) -> AbiPluginResult {
    AbiPluginResult {
        source_id: PLUGIN_ID.into(),
        section: "Showcase".into(),
        title: RString::from(title),
        subtitle: RSome(RString::from(subtitle)),
        icon_path: RNone,
        glyph: RSome(u32::from(glyph)),
        actions: RVec::from(vec![AbiPluginAction {
            label: "Open".into(),
            effect,
        }]),
    }
}

fn grid_view(term: &str) -> AbiView {
    let label = if term.is_empty() { "item" } else { term };

    let items: RVec<AbiGridItem> = (1..=8)
        .map(|i| AbiGridItem {
            id: RString::from(format!("{label}-{i}")),
            title: RString::from(format!("{label} #{i}")),
            subtitle: RNone,
            image: AbiImageSource::None,
        })
        .collect();

    AbiView {
        view_id: "grid-demo".into(),
        title: "Grid Demo".into(),
        search_placeholder: RSome("Filter items…".into()),
        submit_label: RSome("Copy Label".into()),
        body: AbiViewBody::Grid { columns: 4, items },
    }
}

fn detail_view() -> AbiView {
    AbiView {
        view_id: "detail-demo".into(),
        title: "Detail Demo".into(),
        search_placeholder: RNone,
        submit_label: RSome("Close".into()),
        body: AbiViewBody::Detail {
            body: RString::from(
                "This is a detail view rendered entirely from plugin data.\n\n\
                 It supports multiple paragraphs of body text alongside a \
                 metadata sidebar on the right.",
            ),
            metadata: RVec::from(vec![
                AbiKeyValue {
                    key: "Type".into(),
                    value: "Demo".into(),
                },
                AbiKeyValue {
                    key: "Author".into(),
                    value: "example".into(),
                },
                AbiKeyValue {
                    key: "Version".into(),
                    value: "0.1.0".into(),
                },
            ]),
        },
    }
}

fn form_view() -> AbiView {
    AbiView {
        view_id: "snippet-form".into(),
        title: "Create Snippet".into(),
        search_placeholder: RNone,
        submit_label: RSome("Create Snippet".into()),
        body: AbiViewBody::Form {
            fields: RVec::from(vec![
                AbiFormField {
                    id: "name".into(),
                    label: "Name".into(),
                    kind: AbiFieldKind::Text("React Boilerplate".into()),
                },
                AbiFormField {
                    id: "keyword".into(),
                    label: "Keyword".into(),
                    kind: AbiFieldKind::Text("rb".into()),
                },
                AbiFormField {
                    id: "type".into(),
                    label: "Type".into(),
                    kind: AbiFieldKind::Dropdown {
                        options: RVec::from(vec![
                            RString::from("Text"),
                            RString::from("Code"),
                            RString::from("Link"),
                        ]),
                        selected: 0,
                    },
                },
                AbiFormField {
                    id: "auto_expand".into(),
                    label: "Auto-expand".into(),
                    kind: AbiFieldKind::Toggle(true),
                },
                AbiFormField {
                    id: "snippet".into(),
                    label: "Snippet".into(),
                    kind: AbiFieldKind::TextArea("import React from 'react';".into()),
                },
            ]),
        },
    }
}

export_plugin!(ShowcasePlugin);
