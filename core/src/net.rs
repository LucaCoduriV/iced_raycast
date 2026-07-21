//! Minimal network helpers used by the host (e.g. fetching plugin-provided
//! image URLs for grid thumbnails).

use std::io::Read;

use anyhow::Result;

/// Maximum image size we're willing to download for a thumbnail.
const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

/// Download the bytes at `url` (size-capped). Blocking — callers run it off the
/// UI thread.
pub fn fetch_bytes(url: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    ureq::get(url)
        .call()?
        .into_reader()
        .take(MAX_IMAGE_BYTES)
        .read_to_end(&mut bytes)?;
    Ok(bytes)
}
