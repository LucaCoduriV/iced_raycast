//! Decode downloaded image bytes, expanding animated GIFs into frames so the
//! host can play them.

use std::io::Cursor;

use image::{AnimationDecoder, codecs::gif::GifDecoder};

/// A single decoded animation frame as raw RGBA plus its display duration.
pub struct AnimationFrame {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub delay_ms: u32,
}

/// The result of decoding image bytes.
pub enum Decoded {
    /// A still image; the original encoded bytes (host builds one handle).
    Still(Vec<u8>),
    /// An animation with two or more frames.
    Animated(Vec<AnimationFrame>),
}

/// Decode `bytes`. Multi-frame GIFs become [`Decoded::Animated`]; everything
/// else (including single-frame GIFs) stays [`Decoded::Still`].
pub fn decode(bytes: Vec<u8>) -> Decoded {
    if bytes.len() >= 3
        && &bytes[..3] == b"GIF"
        && let Some(frames) = decode_gif(&bytes).filter(|frames| frames.len() > 1)
    {
        return Decoded::Animated(frames);
    }
    Decoded::Still(bytes)
}

fn decode_gif(bytes: &[u8]) -> Option<Vec<AnimationFrame>> {
    let decoder = GifDecoder::new(Cursor::new(bytes)).ok()?;
    let frames = decoder.into_frames().collect_frames().ok()?;

    let decoded = frames
        .into_iter()
        .map(|frame| {
            let (numer, denom) = frame.delay().numer_denom_ms();
            let delay_ms = if denom == 0 { 100 } else { numer / denom };
            let buffer = frame.into_buffer();
            let (width, height) = buffer.dimensions();
            AnimationFrame {
                rgba: buffer.into_raw(),
                width,
                height,
                // Clamp very short/zero delays to something playable.
                delay_ms: delay_ms.max(20),
            }
        })
        .collect();

    Some(decoded)
}
