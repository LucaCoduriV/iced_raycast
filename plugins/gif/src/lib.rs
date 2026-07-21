//! Real GIF search plugin with pagination.
//!
//! Provider is chosen at runtime:
//! - `GIPHY_API_KEY` set  → **Giphy** (all GIFs). Free key: <https://developers.giphy.com>.
//! - otherwise            → **The Finer Gifs Club** (keyless, but only *The
//!   Office (US)* GIFs).
//!
//! Type `gif` to open a searchable grid; type to search; scroll down to load
//! more. Selecting a GIF copies its link. Network calls happen in
//! `handle_event`, which the host runs off the UI thread.

use plugin_api::{
    export_plugin,
    std_types::{RNone, ROption, RSome, RStr, RString, RVec},
    AbiActionEffect, AbiCommand, AbiGridItem, AbiImageSource, AbiView, AbiViewBody, AbiViewEvent,
    AbiViewEventKind, AbiViewResponse, HostPlugin,
};

const PLUGIN_ID: &str = "media.gif";
const VIEW_ID: &str = "gif-grid";
const COLUMNS: u32 = 4;
const PAGE: usize = 12;

#[derive(Default)]
struct GifPlugin;

impl HostPlugin for GifPlugin {
    fn id(&self) -> RString {
        PLUGIN_ID.into()
    }

    fn commands(&self) -> RVec<AbiCommand> {
        RVec::from(vec![AbiCommand {
            id: "search".into(),
            title: "Search GIFs".into(),
            subtitle: RSome(RString::from(Provider::detect().subtitle())),
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
            AbiActionEffect::PushView(grid_view(Provider::detect().title(), RVec::new()))
        } else {
            AbiActionEffect::None
        }
    }

    fn handle_event(&self, event: AbiViewEvent) -> AbiViewResponse {
        if event.view_id.as_str() != VIEW_ID {
            return AbiViewResponse::None;
        }

        match event.kind {
            AbiViewEventKind::Search(term) => {
                AbiViewResponse::Update(search_view(term.as_str().trim()))
            }
            AbiViewEventKind::LoadMore { term, offset } => {
                let items = Provider::detect()
                    .fetch(term.as_str().trim(), offset as usize)
                    .unwrap_or_default();
                AbiViewResponse::Append(items)
            }
            AbiViewEventKind::Activate(id) => {
                AbiViewResponse::Effect(AbiActionEffect::CopyToClipboard(id))
            }
            AbiViewEventKind::Submit(_) => AbiViewResponse::None,
        }
    }
}

fn search_view(term: &str) -> AbiView {
    let provider = Provider::detect();

    match provider.fetch(term, 0) {
        Ok(items) if !items.is_empty() => grid_view(provider.title(), items),
        Ok(_) => message_view(provider.empty_hint(term)),
        Err(error) => message_view(&format!("Couldn't reach the GIF provider: {error}")),
    }
}

// --- Providers --------------------------------------------------------------

enum Provider {
    Giphy(String),
    FinerGifs,
}

impl Provider {
    fn detect() -> Self {
        match std::env::var("GIPHY_API_KEY") {
            Ok(key) if !key.trim().is_empty() => Provider::Giphy(key),
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

fn grid_view(title: &str, items: RVec<AbiGridItem>) -> AbiView {
    AbiView {
        view_id: VIEW_ID.into(),
        title: RString::from(title),
        search_placeholder: RSome("Search GIFs…".into()),
        submit_label: RSome("Copy Link".into()),
        body: AbiViewBody::Grid {
            columns: COLUMNS,
            items,
        },
    }
}

fn message_view(message: &str) -> AbiView {
    grid_view(
        "GIFs",
        RVec::from(vec![AbiGridItem {
            id: String::new().into(),
            title: RString::from(message),
            subtitle: RNone,
            image: AbiImageSource::None,
        }]),
    )
}

export_plugin!(GifPlugin);
