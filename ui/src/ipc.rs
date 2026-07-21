//! Minimal single-instance IPC.
//!
//! A bare `iced_raycast` invocation (e.g. from a compositor keybind) first tries
//! to reach an already-running resident agent over a local socket and ask it to
//! show the launcher — a warm, instant open with everything already loaded. Only
//! if no agent answers does the process become the resident agent itself.

#[cfg(unix)]
pub use unix::{bind, serve, try_show};

#[cfg(unix)]
mod unix {
    use std::io::{Read, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;

    /// The agent's control socket, under `$XDG_RUNTIME_DIR` (falling back to the
    /// temp dir).
    fn socket_path() -> PathBuf {
        std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("iced_raycast.sock")
    }

    /// Client: ask a running agent to show the launcher. Returns `true` if one
    /// was listening and accepted the request.
    pub fn try_show() -> bool {
        match UnixStream::connect(socket_path()) {
            Ok(mut stream) => stream.write_all(b"show").is_ok(),
            Err(_) => false,
        }
    }

    /// Server: claim the control socket, or `None` if another agent already
    /// holds it (so we don't start a second resident process).
    pub fn bind() -> Option<UnixListener> {
        let path = socket_path();
        if UnixStream::connect(&path).is_ok() {
            return None; // a live agent is already listening
        }
        let _ = std::fs::remove_file(&path); // clear a stale socket file
        UnixListener::bind(&path).ok()
    }

    /// Blocking accept loop; invokes `on_show` for each "show" request. Meant to
    /// run on a dedicated thread.
    pub fn serve(listener: UnixListener, mut on_show: impl FnMut()) {
        for stream in listener.incoming().flatten() {
            let mut stream = stream;
            let mut buffer = [0u8; 8];
            if matches!(stream.read(&mut buffer), Ok(read) if buffer[..read].starts_with(b"show")) {
                on_show();
            }
        }
    }
}

// TODO(cross-platform): a Windows named-pipe backend so the resident model works
// there too. Until then, non-unix builds have no resident IPC (each launch is
// standalone) and fall back to the ephemeral single-window app.
#[cfg(not(unix))]
pub use fallback::*;
#[cfg(not(unix))]
mod fallback {
    pub type Listener = ();

    pub fn try_show() -> bool {
        false
    }

    pub fn bind() -> Option<Listener> {
        None
    }

    pub fn serve(_listener: Listener, _on_show: impl FnMut()) {}
}
