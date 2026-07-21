//! Development convenience: copy the bundled plugin `cdylib`s into the user's
//! plugins directory whenever the app is built (debug only), so `cargo run`
//! always picks up freshly-built, ABI-matching plugins.
//!
//! The plugins are declared as `artifact = "cdylib"` build-dependencies, so
//! Cargo builds them first and exposes their paths via `CARGO_CDYLIB_FILE_*`
//! environment variables.

use std::fs;
use std::path::PathBuf;

fn main() {
    // Only auto-install for local dev builds; release artifacts are shipped and
    // installed separately.
    if std::env::var("PROFILE").as_deref() != Ok("debug") {
        return;
    }

    let Some(dest) = plugins_dir() else {
        return;
    };
    if fs::create_dir_all(&dest).is_err() {
        return;
    }

    // Copy every cdylib artifact Cargo built for us.
    for (key, value) in std::env::vars() {
        if !key.starts_with("CARGO_CDYLIB_FILE_") {
            continue;
        }

        let src = PathBuf::from(&value);
        let is_dylib = src
            .extension()
            .is_some_and(|ext| ext == std::env::consts::DLL_EXTENSION);
        if !is_dylib {
            continue;
        }

        if let Some(name) = src.file_name() {
            let _ = fs::copy(&src, dest.join(name));
        }
    }
}

fn plugins_dir() -> Option<PathBuf> {
    // Must match core::plugins::plugins_dir (com / lcvitor / iced_raycast).
    directories::ProjectDirs::from("com", "lcvitor", "iced_raycast")
        .map(|dirs| dirs.data_local_dir().join("plugins"))
}
