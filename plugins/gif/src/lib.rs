//! Real GIF search plugin with pagination.
//!
//! Provider is chosen at runtime:
//! - `GIPHY_API_KEY` set  → **Giphy** (all GIFs). Free key: <https://developers.giphy.com>.
//! - otherwise            → **The Finer Gifs Club** (keyless, but only *The
//!   Office (US)* GIFs).
//!
//! Type `gif` to open a searchable grid; type to search; scroll down to load
//! more. Selecting a GIF copies its link by default, or the GIF's bytes when
//! the `copy_target` preference is set to `GIF`. Network calls happen in
//! `handle_event`, which the host runs off the UI thread.

use std::sync::RwLock;

use plugin_api::{
    export_plugin,
    std_types::{RNone, ROption, RSome, RStr, RString, RVec},
    AbiActionEffect, AbiCommand, AbiGridItem, AbiImageSource, AbiPluginMeta, AbiPreference,
    AbiPreferenceKind, AbiPreferenceValue, AbiView, AbiViewBody, AbiViewEvent, AbiViewEventKind,
    AbiViewResponse, HostPlugin,
};

const PLUGIN_ID: &str = "media.gif";
const VIEW_ID: &str = "gif-grid";
const COLUMNS: u32 = 4;
const PAGE: usize = 12;

/// What the user gets when they activate a GIF.
#[derive(Clone, Copy, PartialEq, Eq)]
enum CopyTarget {
    /// Put the GIF's URL text on the clipboard (default). Small, portable, works
    /// everywhere but pastes as a link.
    Link,
    /// Download the GIF and put its raw bytes on the clipboard as `image/gif`,
    /// so paste-aware apps embed the animation.
    Gif,
}

impl CopyTarget {
    const OPTIONS: &'static [&'static str] = &["Link", "GIF"];

    fn from_index(index: u64) -> Self {
        match index {
            1 => CopyTarget::Gif,
            _ => CopyTarget::Link,
        }
    }

    fn to_index(self) -> u64 {
        match self {
            CopyTarget::Link => 0,
            CopyTarget::Gif => 1,
        }
    }

    fn submit_label(self) -> &'static str {
        match self {
            CopyTarget::Link => "Copy Link",
            CopyTarget::Gif => "Copy GIF",
        }
    }
}

impl Default for CopyTarget {
    fn default() -> Self {
        CopyTarget::Link
    }
}

struct GifPlugin {
    /// Giphy API key set via the settings preference (interior-mutable so
    /// `set_preference`, which takes `&self`, can update it). Empty means unset.
    api_key: RwLock<String>,
    /// Whether Activate copies the URL or downloads the bytes.
    copy_target: RwLock<CopyTarget>,
}

impl Default for GifPlugin {
    fn default() -> Self {
        GifPlugin {
            api_key: RwLock::new(String::new()),
            copy_target: RwLock::new(CopyTarget::default()),
        }
    }
}

impl GifPlugin {
    /// The effective Giphy key: the one set in settings, else `GIPHY_API_KEY`.
    fn effective_key(&self) -> Option<String> {
        let stored = self
            .api_key
            .read()
            .ok()
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty());

        stored.or_else(|| {
            std::env::var("GIPHY_API_KEY")
                .ok()
                .map(|key| key.trim().to_string())
                .filter(|key| !key.is_empty())
        })
    }

    /// The provider chosen by the effective key.
    fn provider(&self) -> Provider {
        Provider::for_key(self.effective_key())
    }

    /// The current copy behavior, defaulting to `Link` if the lock is poisoned.
    fn copy_target(&self) -> CopyTarget {
        self.copy_target
            .read()
            .map(|guard| *guard)
            .unwrap_or_default()
    }
}

impl HostPlugin for GifPlugin {
    fn id(&self) -> RString {
        PLUGIN_ID.into()
    }

    fn metadata(&self) -> AbiPluginMeta {
        AbiPluginMeta {
            name: "GIF Search".into(),
            author: "lcvitor".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            description: "Find and copy the perfect GIF without leaving your keyboard — \
                          search, preview animations inline and paste anywhere."
                .into(),
        }
    }

    fn preferences(&self) -> RVec<AbiPreference> {
        RVec::from(vec![
            AbiPreference {
                id: "giphy_api_key".into(),
                label: "Giphy API key".into(),
                hint: "Paste a Giphy API key to search all GIFs. Without one, a keyless \
                       provider (The Office GIFs) is used."
                    .into(),
                // The current key (masked in the UI). Empty unless the user set
                // one; the `GIPHY_API_KEY` env var is a separate fallback and is
                // not surfaced here.
                kind: AbiPreferenceKind::Secret(
                    self.api_key
                        .read()
                        .map(|key| key.clone())
                        .unwrap_or_default()
                        .into(),
                ),
            },
            AbiPreference {
                id: "rating".into(),
                label: "Rating".into(),
                hint: "Filter out mature results.".into(),
                kind: AbiPreferenceKind::Select {
                    options: RVec::from(vec!["G".into(), "PG".into(), "PG-13".into(), "R".into()]),
                    selected: 2,
                },
            },
            AbiPreference {
                id: "copy_target".into(),
                label: "Copy on activate".into(),
                hint: "Whether pressing Enter copies the GIF's link or the GIF \
                       itself (paste-aware apps embed the animation)."
                    .into(),
                kind: AbiPreferenceKind::Select {
                    options: RVec::from(
                        CopyTarget::OPTIONS
                            .iter()
                            .map(|option| RString::from(*option))
                            .collect::<Vec<_>>(),
                    ),
                    selected: self.copy_target().to_index(),
                },
            },
        ])
    }

    fn set_preference(&self, id: RStr<'_>, value: AbiPreferenceValue) {
        match (id.as_str(), value) {
            ("giphy_api_key", AbiPreferenceValue::Text(key)) => {
                if let Ok(mut guard) = self.api_key.write() {
                    *guard = key.to_string();
                }
            }
            ("copy_target", AbiPreferenceValue::Choice(index)) => {
                if let Ok(mut guard) = self.copy_target.write() {
                    *guard = CopyTarget::from_index(index);
                }
            }
            _ => {}
        }
    }

    fn commands(&self) -> RVec<AbiCommand> {
        RVec::from(vec![AbiCommand {
            id: "search".into(),
            title: "Search GIFs".into(),
            subtitle: RSome(RString::from(self.provider().subtitle())),
            keywords: RVec::from(vec!["gif".into(), "gifs".into(), "giphy".into()]),
            icon_path: RNone,
            glyph: RSome(u32::from('G')),
            category: "GIF".into(),
            needs_argument: false,
            argument_placeholder: RNone,
            fallback: false,
        }])
    }

    fn run_command(&self, command_id: RStr<'_>, _argument: ROption<RString>) -> AbiActionEffect {
        if command_id.as_str() == "search" {
            AbiActionEffect::PushView(grid_view(
                self.provider().title(),
                self.copy_target().submit_label(),
                RVec::new(),
            ))
        } else {
            AbiActionEffect::None
        }
    }

    fn handle_event(&self, event: AbiViewEvent) -> AbiViewResponse {
        if event.view_id.as_str() != VIEW_ID {
            return AbiViewResponse::None;
        }

        match event.kind {
            AbiViewEventKind::Search(term) => AbiViewResponse::Update(search_view(
                term.as_str().trim(),
                &self.provider(),
                self.copy_target().submit_label(),
            )),
            AbiViewEventKind::LoadMore { term, offset } => {
                let items = self
                    .provider()
                    .fetch(term.as_str().trim(), offset as usize)
                    .unwrap_or_default();
                AbiViewResponse::Append(items)
            }
            AbiViewEventKind::Activate(id) => {
                AbiViewResponse::Effect(match self.copy_target() {
                    CopyTarget::Link => AbiActionEffect::CopyToClipboard(id),
                    CopyTarget::Gif => AbiActionEffect::CopyImageFromUrl {
                        url: id,
                        mime: RString::from("image/gif"),
                    },
                })
            }
            AbiViewEventKind::Submit(_) => AbiViewResponse::None,
        }
    }
}

fn search_view(term: &str, provider: &Provider, submit_label: &str) -> AbiView {
    match provider.fetch(term, 0) {
        Ok(items) if !items.is_empty() => grid_view(provider.title(), submit_label, items),
        Ok(_) => message_view(provider.empty_hint(term), submit_label),
        Err(error) => message_view(
            &format!("Couldn't reach the GIF provider: {error}"),
            submit_label,
        ),
    }
}

// --- Providers --------------------------------------------------------------

enum Provider {
    Giphy(String),
    FinerGifs,
}

impl Provider {
    fn for_key(key: Option<String>) -> Self {
        match key {
            Some(key) if !key.trim().is_empty() => Provider::Giphy(key),
            _ => Provider::FinerGifs,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Provider::Giphy(_) => "GIFs",
            Provider::FinerGifs => "The Office GIFs",
        }
    }

    fn subtitle(&self) -> &'static str {
        match self {
            Provider::Giphy(_) => "Search all GIFs (Giphy)",
            Provider::FinerGifs => "The Office · set GIPHY_API_KEY for all GIFs",
        }
    }

    fn empty_hint(&self, term: &str) -> &'static str {
        if !term.is_empty() {
            return "No GIFs found. Try different words.";
        }
        match self {
            Provider::Giphy(_) => "Type to search GIFs.",
            Provider::FinerGifs => {
                "Type to search The Office GIFs. Set GIPHY_API_KEY for all GIFs."
            }
        }
    }

    fn fetch(&self, term: &str, offset: usize) -> Result<RVec<AbiGridItem>, String> {
        match self {
            Provider::Giphy(key) => giphy(key, term, offset),
            Provider::FinerGifs => finer_gifs(term, offset),
        }
    }
}

fn giphy(key: &str, term: &str, offset: usize) -> Result<RVec<AbiGridItem>, String> {
    let base = if term.is_empty() {
        "https://api.giphy.com/v1/gifs/trending"
    } else {
        "https://api.giphy.com/v1/gifs/search"
    };

    let mut request = ureq::get(base)
        .query("api_key", key)
        .query("limit", &PAGE.to_string())
        .query("offset", &offset.to_string());
    if !term.is_empty() {
        request = request.query("q", term);
    }

    let json: serde_json::Value = request
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())?;

    let data = json["data"].as_array().cloned().unwrap_or_default();

    Ok(data
        .iter()
        .filter_map(|gif| {
            let images = &gif["images"];
            let thumb = giphy_thumb(images)?;
            let link = images["original"]["url"]
                .as_str()
                .or_else(|| gif["url"].as_str())
                .unwrap_or(&thumb)
                .to_string();
            let title = gif["title"].as_str().unwrap_or("GIF").trim();

            Some(AbiGridItem {
                id: RString::from(link),
                title: RString::from(if title.is_empty() { "GIF" } else { title }),
                subtitle: RNone,
                image: AbiImageSource::Url(RString::from(thumb)),
            })
        })
        .collect())
}

/// Pick the smallest reasonable animated thumbnail Giphy offers.
fn giphy_thumb(images: &serde_json::Value) -> Option<String> {
    for key in [
        "fixed_width_small",
        "fixed_width_downsampled",
        "fixed_width",
        "downsized",
        "original",
    ] {
        if let Some(url) = images[key]["url"].as_str() {
            if !url.is_empty() {
                return Some(url.to_string());
            }
        }
    }
    None
}

fn finer_gifs(term: &str, offset: usize) -> Result<RVec<AbiGridItem>, String> {
    if term.is_empty() {
        return Ok(RVec::new());
    }

    let json: serde_json::Value = ureq::get("https://api.thefinergifs.club/search")
        .query("q", term)
        .query("q.parser", "simple")
        .query("sort", "_score desc")
        .query("size", &PAGE.to_string())
        .query("start", &offset.to_string())
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())?;

    let results = json["results"].as_array().cloned().unwrap_or_default();

    Ok(results
        .iter()
        .filter_map(|entry| {
            let fields = &entry["fields"];
            let fileid = fields["fileid"].as_str()?;
            let title = fields["text"].as_str().unwrap_or("GIF").trim();
            let url = format!("https://media.thefinergifs.club/{fileid}.gif");

            Some(AbiGridItem {
                id: RString::from(url.clone()),
                title: RString::from(if title.is_empty() { "GIF" } else { title }),
                subtitle: RNone,
                image: AbiImageSource::Url(RString::from(url)),
            })
        })
        .collect())
}

// --- View helpers -----------------------------------------------------------

fn grid_view(title: &str, submit_label: &str, items: RVec<AbiGridItem>) -> AbiView {
    AbiView {
        view_id: VIEW_ID.into(),
        title: RString::from(title),
        search_placeholder: RSome("Search GIFs…".into()),
        submit_label: RSome(RString::from(submit_label)),
        body: AbiViewBody::Grid {
            columns: COLUMNS,
            items,
        },
    }
}

fn message_view(message: &str, submit_label: &str) -> AbiView {
    grid_view(
        "GIFs",
        submit_label,
        RVec::from(vec![AbiGridItem {
            id: String::new().into(),
            title: RString::from(message),
            subtitle: RNone,
            image: AbiImageSource::None,
        }]),
    )
}

export_plugin!(GifPlugin);
