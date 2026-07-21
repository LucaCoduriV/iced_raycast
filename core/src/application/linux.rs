use std::{
    collections::HashSet,
    os::unix::process::CommandExt,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};
use freedesktop_desktop_entry::{
    DesktopEntry, current_desktop, desktop_entries, get_languages_from_env,
};
use linicon::lookup_icon;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use which::which;

use crate::application::{Application, Image};

struct TerminalProfile {
    exe: &'static str,
    flag: &'static str,
}

static TERMINALS: [TerminalProfile; 6] = [
    TerminalProfile {
        exe: "ghostty",
        flag: "-e",
    },
    TerminalProfile {
        exe: "kitty",
        flag: "-e",
    },
    TerminalProfile {
        exe: "alacritty",
        flag: "-e",
    },
    TerminalProfile {
        exe: "termite",
        flag: "-e",
    },
    TerminalProfile {
        exe: "gnome-terminal",
        flag: "--",
    },
    TerminalProfile {
        exe: "weston-terminal",
        flag: "--",
    },
];

fn get_terminal() -> Option<&'static TerminalProfile> {
    TERMINALS.iter().find(|t| which(t.exe).is_ok())
}

#[derive(Debug, Clone)]
pub struct LinuxApplication {
    name: String,
    description: Option<String>,
    exec: Vec<String>,
    working_dir: Option<String>,
    icon_path: Option<String>,
    is_terminal: bool,
}

impl LinuxApplication {
    fn from_desktop_entry(entry: &DesktopEntry, locales: &[String]) -> Option<Self> {
        // Spec-compliant parsing: honours quoting and expands/strips field
        // codes (%f, %u, %i, ...) instead of naively splitting on whitespace.
        let exec = entry.parse_exec().ok()?;

        if exec.is_empty() {
            return None;
        }

        let name = entry
            .name(locales)
            .map(|cow| cow.into_owned())
            .unwrap_or_else(|| "Unknown".to_string());

        // Prefer the human-friendly Comment, falling back to GenericName
        // (e.g. "Web Browser") so entries always carry a useful subtitle.
        let description = entry
            .comment(locales)
            .or_else(|| entry.generic_name(locales))
            .map(|cow| cow.into_owned())
            .filter(|s| !s.is_empty());

        let icon_path = entry.icon().and_then(find_icon);

        Some(LinuxApplication {
            name,
            description,
            exec,
            working_dir: entry.path().map(str::to_string).filter(|s| !s.is_empty()),
            icon_path,
            is_terminal: entry.terminal(),
        })
    }
}

/// Whether a desktop entry should surface as a launchable application,
/// following the freedesktop desktop-entry visibility rules.
fn should_include(entry: &DesktopEntry, current_desktops: Option<&[String]>) -> bool {
    // Only real applications are launchable (skip Link / Directory entries).
    // Entries omitting Type are treated leniently.
    if let Some(kind) = entry.type_()
        && kind != "Application"
    {
        return false;
    }

    // NoDisplay = "don't show in menus"; Hidden = "treat as deleted".
    if entry.no_display() || entry.hidden() {
        return false;
    }

    // TryExec names a binary that must exist for the entry to be usable.
    if let Some(try_exec) = entry.try_exec()
        && !try_exec_available(try_exec)
    {
        return false;
    }

    // Respect OnlyShowIn / NotShowIn against the running desktop. When the
    // current desktop is unknown we stay lenient and show the entry.
    if let Some(current) = current_desktops {
        let in_current =
            |envs: Vec<&str>| envs.iter().any(|e| current.iter().any(|c| c.eq_ignore_ascii_case(e)));

        if let Some(only) = entry.only_show_in()
            && !in_current(only)
        {
            return false;
        }

        if let Some(not) = entry.not_show_in()
            && in_current(not)
        {
            return false;
        }
    }

    true
}

/// A `TryExec` value is satisfied if it resolves to an existing executable:
/// an absolute/relative path that exists, or a bare name found on `PATH`.
fn try_exec_available(try_exec: &str) -> bool {
    if try_exec.contains('/') {
        Path::new(try_exec).exists()
    } else {
        which(try_exec).is_ok()
    }
}

fn find_icon(icon_name: &str) -> Option<String> {
    let path = Path::new(icon_name);

    if path.is_absolute() {
        if path.exists() {
            return Some(icon_name.to_string());
        } else {
            let stem = path.file_stem()?.to_str()?;
            return find_icon(stem);
        }
    }

    let found_by_theme = lookup_icon(icon_name)
        .use_fallback_themes(true)
        .filter_map(|e| e.ok())
        .filter_map(|x| x.path.into_os_string().into_string().ok())
        .next();

    if let Some(path) = found_by_theme {
        return Some(path);
    }

    let fallback_dirs = [
        "/usr/share/pixmaps",
        "/usr/share/icons",
        "/usr/share/icons/hicolor/48x48/apps",
        "/usr/share/icons/hicolor/scalable/apps",
    ];

    let extensions = ["", ".png", ".svg", ".xpm", ".ico"];

    for dir in fallback_dirs {
        for ext in extensions {
            let mut candidate = std::path::PathBuf::from(dir);
            candidate.push(format!("{}{}", icon_name, ext));

            if candidate.exists() {
                return candidate.into_os_string().into_string().ok();
            }
        }
    }

    None
}

impl Application for LinuxApplication {
    fn name(&self) -> &str {
        &self.name
    }

    fn alias(&self) -> Option<&str> {
        None
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn execute(&self, _arg: Option<String>) -> Result<()> {
        if self.exec.is_empty() {
            return Ok(());
        }

        let binary = &self.exec[0];
        let args = &self.exec[1..];

        let mut cmd = if self.is_terminal {
            if let Some(term_profile) = get_terminal() {
                let mut c = Command::new(term_profile.exe);
                c.arg(term_profile.flag);
                c.arg(binary);
                c.args(args);
                c
            } else {
                let mut c = Command::new(binary);
                c.args(args);
                c
            }
        } else {
            let mut c = Command::new(binary);
            c.args(args);
            c
        };

        if let Some(dir) = &self.working_dir {
            cmd.current_dir(dir);
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0);

        let _ = cmd
            .spawn()
            .with_context(|| format!("Failed to launch {}", self.name))?;

        Ok(())
    }

    fn icon(&self) -> Option<Image> {
        self.icon_path.as_ref().map(|v| Image::Path(v.clone()))
    }

    fn lookup_applications() -> Vec<Self>
    where
        Self: Sized,
    {
        let locales = get_languages_from_env();
        let current_desktops = current_desktop();

        // `desktop_entries` yields every .desktop file across all XDG data
        // dirs in precedence order (XDG_DATA_HOME first). Filter for visible
        // applications, then keep only the first entry seen per app id so a
        // user override in ~/.local/share shadows the system copy.
        let mut seen_ids = HashSet::new();
        let visible: Vec<DesktopEntry> = desktop_entries(&locales)
            .into_iter()
            .filter(|entry| should_include(entry, current_desktops.as_deref()))
            .filter(|entry| seen_ids.insert(entry.id().to_string()))
            .collect();

        // Icon resolution touches the filesystem per entry, so parallelise it.
        // `collect` preserves the precedence order established above.
        let apps: Vec<LinuxApplication> = visible
            .into_par_iter()
            .filter_map(|entry| LinuxApplication::from_desktop_entry(&entry, &locales))
            .collect();

        // Collapse entries that are indistinguishable to the user — same
        // display name launching the same command (e.g. `google-chrome` and
        // `google-chrome-stable`). Distinct ids with differing commands stay.
        let mut seen = HashSet::new();
        apps.into_iter()
            .filter(|app| seen.insert((app.name.clone(), app.exec.clone())))
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::LinuxApplication;
    use crate::application::Application;
    use std::collections::HashSet;

    #[test]
    fn get_applications() {
        let apps = LinuxApplication::lookup_applications();
        println!("discovered {} applications", apps.len());

        // No two surfaced apps should share a display name + command.
        let unique: HashSet<(&str, &Vec<String>)> =
            apps.iter().map(|a| (a.name(), &a.exec)).collect();
        assert_eq!(
            unique.len(),
            apps.len(),
            "duplicate applications were not collapsed"
        );

        let with_desc = apps.iter().filter(|a| a.description().is_some()).count();
        println!("{with_desc} of them carry a description");

        for app in apps.iter().take(5) {
            println!("- {} — {:?}", app.name(), app.description());
        }
    }
}
