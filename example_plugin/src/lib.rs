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
    std_types::{RNone, ROption, RSome, RStr, RString, RVec},
    AbiActionEffect, AbiCommand, AbiFieldKind, AbiFieldValueKind, AbiFormField, AbiGridItem,
    AbiImageSource, AbiKeyValue, AbiView, AbiViewBody, AbiViewEvent, AbiViewEventKind,
    AbiViewResponse, HostPlugin,
};

const PLUGIN_ID: &str = "example.showcase";

#[derive(Default)]
struct ShowcasePlugin;

impl HostPlugin for ShowcasePlugin {
    fn id(&self) -> RString {
        PLUGIN_ID.into()
    }

    fn commands(&self) -> RVec<AbiCommand> {
        RVec::from(vec![
            command(
                "grid",
                "Grid Demo",
                "Open a searchable grid view",
                'G',
                &["grid"],
                false,
            ),
            command(
                "detail",
                "Detail Demo",
                "Open a detail view",
                'D',
                &["detail"],
                false,
            ),
            command(
                "form",
                "Create Snippet",
                "Open a form view",
                'F',
                &["form", "snippet"],
                false,
            ),
            command(
                "uppercase",
                "Uppercase Text",
                "Copy your text uppercased",
                'A',
                &["uppercase", "upper"],
                true,
            ),
        ])
    }

    fn run_command(&self, command_id: RStr<'_>, argument: ROption<RString>) -> AbiActionEffect {
        match command_id.as_str() {
            "grid" => AbiActionEffect::PushView(grid_view("")),
            "detail" => AbiActionEffect::PushView(detail_view()),
            "form" => AbiActionEffect::PushView(form_view()),
            "uppercase" => {
                let text = argument
                    .into_option()
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                AbiActionEffect::CopyToClipboard(RString::from(text.to_uppercase()))
            }
            _ => AbiActionEffect::None,
        }
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

fn command(
    id: &str,
    title: &str,
    subtitle: &str,
    glyph: char,
    keywords: &[&str],
    needs_argument: bool,
) -> AbiCommand {
    AbiCommand {
        id: RString::from(id),
        title: RString::from(title),
        subtitle: RSome(RString::from(subtitle)),
        keywords: keywords.iter().map(|k| RString::from(*k)).collect(),
        icon_path: RNone,
        glyph: RSome(u32::from(glyph)),
        category: "Demo".into(),
        needs_argument,
        argument_placeholder: if needs_argument {
            RSome("Text to uppercase…".into())
        } else {
            RNone
        },
        fallback: false,
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
