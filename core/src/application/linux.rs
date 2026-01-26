use std::{
    os::unix::process::CommandExt,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result};
use freedesktop_desktop_entry::{DesktopEntry, desktop_entries, get_languages_from_env};
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
    exec: Vec<String>,
    icon_path: Option<String>,
    is_terminal: bool,
}

impl LinuxApplication {
    fn from_desktop_entry(entry: &DesktopEntry, locales: &[String]) -> Option<Self> {
        let exec_raw = entry.exec()?;

        let exec: Vec<String> = exec_raw
            .split_whitespace()
            .map(|s| {
                s.replace("%f", "")
                    .replace("%F", "")
                    .replace("%u", "")
                    .replace("%U", "")
                    .replace("%i", "")
                    .replace("%c", "")
                    .replace("%k", "")
            })
            .filter(|s| !s.is_empty())
            .collect();

        if exec.is_empty() {
            return None;
        }

        let name = entry
            .name(locales)
            .map(|cow| cow.into_owned())
            .unwrap_or_else(|| "Unknown".to_string());

        let icon_path = entry.icon().and_then(find_icon).or_else(|| {
            println!("{} | {:?}", name, entry.icon());
            None
        });

        Some(LinuxApplication {
            name,
            exec,
            icon_path,
            is_terminal: entry.terminal(),
        })
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
        None
    }

    fn execute(&self, _arg: Option<&str>) -> Result<()> {
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

        let entries = desktop_entries(&locales);

        entries
            .into_par_iter()
            .filter_map(|entry| {
                if entry.no_display() {
                    return None;
                }
                LinuxApplication::from_desktop_entry(&entry, &locales)
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::LinuxApplication;
    use crate::application::Application;

    #[test]
    fn get_applications() {
        let apps = LinuxApplication::lookup_applications();
        println!("{:?}", apps.len());
    }
}
