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
    std_types::{RBox, RStr, RString},
};
use directories::ProjectDirs;
use plugin_api::{
    AbiActionEffect, AbiFieldValue, AbiFieldValueKind, AbiFormField, AbiGridItem, AbiImageSource,
    AbiPluginResult, AbiView, AbiViewBody, AbiViewEvent, AbiViewEventKind, AbiViewResponse,
    HostPlugin_TO, PluginModRef,
};

use super::{
    ActionEffect, FieldKind, FieldValue, FieldValueKind, FormField, GridItem, ImageSource,
    KeyValue, Plugin, PluginAction, PluginResult, View, ViewBody, ViewEvent, ViewEventKind,
    ViewResponse,
};
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

    fn handle_event(&self, event: ViewEvent) -> ViewResponse {
        convert_response(self.inner.handle_event(to_abi_event(event)))
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
        assert!(
            quiet.is_empty(),
            "example plugin fired on an unrelated query"
        );

        // The example plugin's "grid" result pushes a grid view...
        let example = plugins
            .iter()
            .find(|p| p.id() == "example.showcase")
            .expect("example plugin not loaded");
        let grid = example
            .query("grid")
            .into_iter()
            .next()
            .expect("no grid result");
        assert!(
            matches!(
                grid.actions.first().map(|a| &a.effect),
                Some(ActionEffect::PushView(view)) if matches!(view.body, ViewBody::Grid { .. })
            ),
            "grid result did not push a grid view"
        );

        // ...and searching in it round-trips over the ABI to an updated grid.
        match example.handle_event(ViewEvent {
            view_id: "grid-demo".to_string(),
            kind: ViewEventKind::Search("cats".to_string()),
        }) {
            ViewResponse::Update(View {
                body: ViewBody::Grid { items, .. },
                ..
            }) => {
                assert_eq!(items.len(), 8)
            }
            _ => panic!("expected a grid Update from the grid search"),
        }
    }

    /// The Google plugin opens a browser URL. Same opt-in env var.
    #[test]
    fn google_plugin_opens_url() {
        let Ok(dir) = std::env::var("ICED_RAYCAST_TEST_PLUGIN_DIR") else {
            return;
        };
        let plugins = load_plugins_from_dir(Path::new(&dir));
        let Some(google) = plugins.iter().find(|p| p.id() == "web.google") else {
            return;
        };

        let result = google
            .query("g rust iced")
            .into_iter()
            .next()
            .expect("no result");
        match result.actions.first().map(|a| &a.effect) {
            Some(ActionEffect::OpenUrl(url)) => {
                assert!(
                    url.contains("google.com/search?q=rust+iced"),
                    "unexpected url: {url}"
                );
            }
            other => panic!("expected an OpenUrl effect, got {other:?}"),
        }
    }

    /// The GIF plugin loads and degrades gracefully without an API key.
    #[test]
    fn gif_plugin_pushes_grid() {
        let Ok(dir) = std::env::var("ICED_RAYCAST_TEST_PLUGIN_DIR") else {
            return;
        };
        let plugins = load_plugins_from_dir(Path::new(&dir));
        let Some(gif) = plugins.iter().find(|p| p.id() == "media.gif") else {
            return;
        };

        let result = gif.query("gif").into_iter().next().expect("no gif result");
        assert!(matches!(
            result.actions.first().map(|a| &a.effect),
            Some(ActionEffect::PushView(view)) if matches!(view.body, ViewBody::Grid { .. })
        ));

        // An empty search returns a (message) grid rather than erroring.
        assert!(matches!(
            gif.handle_event(ViewEvent {
                view_id: "gif-grid".to_string(),
                kind: ViewEventKind::Search(String::new()),
            }),
            ViewResponse::Update(View {
                body: ViewBody::Grid { .. },
                ..
            })
        ));

        // Live network path: opt-in via ICED_RAYCAST_TEST_NETWORK.
        if std::env::var_os("ICED_RAYCAST_TEST_NETWORK").is_some() {
            let page1 = match gif.handle_event(ViewEvent {
                view_id: "gif-grid".to_string(),
                kind: ViewEventKind::Search("boss".to_string()),
            }) {
                ViewResponse::Update(View {
                    body: ViewBody::Grid { items, .. },
                    ..
                }) => {
                    assert!(!items.is_empty(), "live gif search returned no items");
                    assert!(
                        items.iter().any(
                            |i| matches!(&i.image, ImageSource::Url(u) if u.ends_with(".gif"))
                        ),
                        "gif items missing image URLs"
                    );
                    items.len()
                }
                _ => panic!("expected a grid from live gif search"),
            };

            // Pagination: a LoadMore at the page-1 offset returns more items.
            match gif.handle_event(ViewEvent {
                view_id: "gif-grid".to_string(),
                kind: ViewEventKind::LoadMore {
                    term: "boss".to_string(),
                    offset: page1 as u64,
                },
            }) {
                ViewResponse::Append(items) => {
                    assert!(!items.is_empty(), "load more returned no items");
                }
                _ => panic!("expected an Append from load more"),
            }
        }
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
        actions: result.actions.into_iter().map(convert_action).collect(),
    }
}

fn convert_action(action: plugin_api::AbiPluginAction) -> PluginAction {
    PluginAction {
        label: action.label.to_string(),
        effect: convert_effect(action.effect),
    }
}

// --- ABI -> native (results coming from the plugin) -------------------------

fn convert_effect(effect: AbiActionEffect) -> ActionEffect {
    match effect {
        AbiActionEffect::CopyToClipboard(text) => ActionEffect::CopyToClipboard(text.to_string()),
        AbiActionEffect::OpenUrl(url) => ActionEffect::OpenUrl(url.to_string()),
        AbiActionEffect::PushView(view) => ActionEffect::PushView(convert_view(view)),
        AbiActionEffect::Close => ActionEffect::Close,
    }
}

fn convert_view(view: AbiView) -> View {
    View {
        view_id: view.view_id.to_string(),
        title: view.title.to_string(),
        search_placeholder: view.search_placeholder.into_option().map(|s| s.to_string()),
        submit_label: view.submit_label.into_option().map(|s| s.to_string()),
        body: match view.body {
            AbiViewBody::Grid { columns, items } => ViewBody::Grid {
                columns,
                items: items.into_iter().map(convert_grid_item).collect(),
            },
            AbiViewBody::Detail { body, metadata } => ViewBody::Detail {
                body: body.to_string(),
                metadata: metadata
                    .into_iter()
                    .map(|kv| KeyValue {
                        key: kv.key.to_string(),
                        value: kv.value.to_string(),
                    })
                    .collect(),
            },
            AbiViewBody::Form { fields } => ViewBody::Form {
                fields: fields.into_iter().map(convert_form_field).collect(),
            },
        },
    }
}

fn convert_grid_item(item: AbiGridItem) -> GridItem {
    GridItem {
        id: item.id.to_string(),
        title: item.title.to_string(),
        subtitle: item.subtitle.into_option().map(|s| s.to_string()),
        image: match item.image {
            AbiImageSource::None => ImageSource::None,
            AbiImageSource::Path(path) => ImageSource::Path(path.to_string()),
            AbiImageSource::Bytes(bytes) => ImageSource::Bytes(bytes.into()),
            AbiImageSource::Url(url) => ImageSource::Url(url.to_string()),
        },
    }
}

fn convert_form_field(field: AbiFormField) -> FormField {
    FormField {
        id: field.id.to_string(),
        label: field.label.to_string(),
        kind: match field.kind {
            plugin_api::AbiFieldKind::Text(v) => FieldKind::Text(v.to_string()),
            plugin_api::AbiFieldKind::TextArea(v) => FieldKind::TextArea(v.to_string()),
            plugin_api::AbiFieldKind::Toggle(v) => FieldKind::Toggle(v),
            plugin_api::AbiFieldKind::Dropdown { options, selected } => FieldKind::Dropdown {
                options: options.into_iter().map(|s| s.to_string()).collect(),
                selected,
            },
        },
    }
}

fn convert_response(response: AbiViewResponse) -> ViewResponse {
    match response {
        AbiViewResponse::None => ViewResponse::None,
        AbiViewResponse::Update(view) => ViewResponse::Update(convert_view(view)),
        AbiViewResponse::Append(items) => {
            ViewResponse::Append(items.into_iter().map(convert_grid_item).collect())
        }
        AbiViewResponse::Effect(effect) => ViewResponse::Effect(convert_effect(effect)),
    }
}

// --- native -> ABI (events going to the plugin) -----------------------------

fn to_abi_event(event: ViewEvent) -> AbiViewEvent {
    AbiViewEvent {
        view_id: RString::from(event.view_id),
        kind: match event.kind {
            ViewEventKind::Search(text) => AbiViewEventKind::Search(RString::from(text)),
            ViewEventKind::Activate(id) => AbiViewEventKind::Activate(RString::from(id)),
            ViewEventKind::Submit(values) => {
                AbiViewEventKind::Submit(values.into_iter().map(to_abi_field_value).collect())
            }
            ViewEventKind::LoadMore { term, offset } => AbiViewEventKind::LoadMore {
                term: RString::from(term),
                offset,
            },
        },
    }
}

fn to_abi_field_value(value: FieldValue) -> AbiFieldValue {
    AbiFieldValue {
        id: RString::from(value.id),
        value: match value.value {
            FieldValueKind::Text(text) => AbiFieldValueKind::Text(RString::from(text)),
            FieldValueKind::Toggle(on) => AbiFieldValueKind::Toggle(on),
            FieldValueKind::Choice(index) => AbiFieldValueKind::Choice(index),
        },
    }
}
