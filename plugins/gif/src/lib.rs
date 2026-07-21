//! Real GIF search plugin, backed by The Finer Gifs Club — a keyless public
//! API (an archive of *The Office (US)* GIFs, searchable by dialogue).
//!
//! Type `gif` to open a searchable grid, then type to search. Selecting a GIF
//! copies its link. No API key required.
//!
//! Network calls happen in `handle_event`, which the host runs off the UI
//! thread, so searching never blocks rendering.

use std::io::Read;

use plugin_api::{
    export_plugin,
    std_types::{RNone, RSome, RStr, RString, RVec},
    AbiActionEffect, AbiGridItem, AbiImageSource, AbiPluginAction, AbiPluginResult, AbiView,
    AbiViewBody, AbiViewEvent, AbiViewEventKind, AbiViewResponse, HostPlugin,
};

const PLUGIN_ID: &str = "media.gif";
const VIEW_ID: &str = "gif-grid";
const COLUMNS: u32 = 4;
const LIMIT: usize = 8;
const SEARCH_URL: &str = "https://api.thefinergifs.club/search";
const MEDIA_BASE: &str = "https://media.thefinergifs.club";

#[derive(Default)]
struct GifPlugin;

impl HostPlugin for GifPlugin {
    fn id(&self) -> RString {
        PLUGIN_ID.into()
    }

    fn query(&self, query: RStr<'_>) -> RVec<AbiPluginResult> {
        let query = query.as_str().trim();
        if query != "gif" && query != "gifs" {
            return RVec::new();
        }

        // The default action opens an (initially empty) searchable grid; the
        // host then asks us to fill it via a Search event.
        RVec::from(vec![AbiPluginResult {
            source_id: PLUGIN_ID.into(),
            section: "Media".into(),
            title: "Search GIFs".into(),
            subtitle: RSome("The Office GIFs from thefinergifs.club".into()),
            icon_path: RNone,
            glyph: RSome(u32::from('G')),
            actions: RVec::from(vec![AbiPluginAction {
                label: "Open".into(),
                effect: AbiActionEffect::PushView(grid_view(RVec::new())),
            }]),
        }])
    }

    fn handle_event(&self, event: AbiViewEvent) -> AbiViewResponse {
        if event.view_id.as_str() != VIEW_ID {
            return AbiViewResponse::None;
        }

        match event.kind {
            AbiViewEventKind::Search(term) => AbiViewResponse::Update(search(term.as_str().trim())),
            // The grid cell id carries the GIF's link; copy it.
            AbiViewEventKind::Activate(id) => {
                AbiViewResponse::Effect(AbiActionEffect::CopyToClipboard(id))
            }
            AbiViewEventKind::Submit(_) => AbiViewResponse::None,
        }
    }
}

fn search(term: &str) -> AbiView {
    if term.is_empty() {
        return message_view("Type to search The Office GIFs.");
    }

    match fetch(term) {
        Ok(items) if !items.is_empty() => grid_view(items),
        Ok(_) => message_view("No GIFs found. Try different words from a line of dialogue."),
        Err(error) => message_view(&format!("Couldn't reach thefinergifs.club: {error}")),
    }
}

fn fetch(term: &str) -> Result<RVec<AbiGridItem>, String> {
    let json: serde_json::Value = ureq::get(SEARCH_URL)
        .query("q", term)
        .query("q.parser", "simple")
        .query("sort", "_score desc")
        .query("size", &LIMIT.to_string())
        .call()
        .map_err(|e| e.to_string())?
        .into_json()
        .map_err(|e| e.to_string())?;

    let results = json["results"].as_array().cloned().unwrap_or_default();

    let items = results
        .iter()
        .take(LIMIT)
        .filter_map(|entry| {
            let fields = &entry["fields"];
            let fileid = fields["fileid"].as_str()?;
            let title = fields["text"].as_str().unwrap_or("GIF").trim();
            let url = format!("{MEDIA_BASE}/{fileid}.gif");

            let image = match download(&url) {
                Some(bytes) => AbiImageSource::Bytes(bytes.into()),
                None => AbiImageSource::None,
            };

            Some(AbiGridItem {
                id: RString::from(url),
                title: RString::from(if title.is_empty() { "GIF" } else { title }),
                subtitle: RNone,
                image,
            })
        })
        .collect();

    Ok(items)
}

/// Download raw image bytes for a thumbnail (best-effort, size-capped).
fn download(url: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    ureq::get(url)
        .call()
        .ok()?
        .into_reader()
        .take(8 * 1024 * 1024)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}

fn grid_view(items: RVec<AbiGridItem>) -> AbiView {
    AbiView {
        view_id: VIEW_ID.into(),
        title: "GIFs".into(),
        search_placeholder: RSome("Search GIFs…".into()),
        submit_label: RSome("Copy Link".into()),
        body: AbiViewBody::Grid {
            columns: COLUMNS,
            items,
        },
    }
}

/// A grid view still (so it keeps its search bar) whose single message cell
/// explains an empty/error state.
fn message_view(message: &str) -> AbiView {
    grid_view(RVec::from(vec![AbiGridItem {
        id: String::new().into(),
        title: RString::from(message),
        subtitle: RNone,
        image: AbiImageSource::None,
    }]))
}

export_plugin!(GifPlugin);
