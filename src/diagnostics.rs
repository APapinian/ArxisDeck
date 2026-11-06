// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! About & diagnostics (ROADMAP.md M1 task 7; PROJECT.md §5.5: troubleshooting view,
//! environment diagnostics, export with personal paths redacted; M7 task 5).
//!
//! Everything is read-only. The home folder is shown as `~` (`logging::redact`); no tokens
//! or passwords exist in Arxis Deck, so nothing else needs hiding.

use crate::logging::redact;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize, Debug)]
pub struct Row {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Debug)]
pub struct Section {
    pub title: String,
    pub rows: Vec<Row>,
}

/// What only the caller knows (UI thread state).
pub struct Context {
    pub backend_mode: String,
    pub backend_status: String,
    pub controllers: Vec<String>,
    pub window: (i32, i32),
    pub qt_version: String,
    pub history_path: Option<PathBuf>,
    pub settings_path: Option<PathBuf>,
}

fn row(key: &str, value: impl Into<String>) -> Row {
    Row {
        key: key.into(),
        value: value.into(),
    }
}

/// `NAME VERSION_ID` from os-release (the host's copy when inside the Flatpak sandbox).
pub fn os_name() -> String {
    for p in ["/run/host/os-release", "/etc/os-release"] {
        if let Ok(text) = std::fs::read_to_string(p) {
            let get = |k: &str| {
                text.lines()
                    .find_map(|l| l.strip_prefix(&format!("{k}=")))
                    .map(|v| v.trim_matches('"').to_string())
                    .unwrap_or_default()
            };
            let name = get("PRETTY_NAME");
            if !name.is_empty() {
                return name;
            }
            return format!("{} {}", get("NAME"), get("VERSION_ID")).trim().to_string();
        }
    }
    "unknown".into()
}

fn size_of(p: &Path) -> String {
    match std::fs::metadata(p) {
        Ok(m) => format!("{} · {:.1} KB", p.display(), m.len() as f64 / 1024.0),
        Err(_) => format!("{} · not created yet", p.display()),
    }
}

/// The last `n` lines of the log file.
pub fn log_tail(n: usize) -> Vec<String> {
    let Some(p) = crate::logging::log_dir().map(|d| d.join("arxis-deck.log")) else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(p) else {
        return Vec::new();
    };
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

pub fn collect(ctx: &Context) -> Vec<Section> {
    let env = |k: &str| std::env::var(k).ok();
    let gaming = env("GAMESCOPE_WAYLAND_DISPLAY").is_some() || env("SteamDeck").as_deref() == Some("1");
    let mut out = vec![
        Section {
            title: "Arxis Deck".into(),
            rows: vec![
                row(
                    "Version",
                    format!(
                        "{} ({})",
                        env!("CARGO_PKG_VERSION"),
                        if cfg!(debug_assertions) {
                            "debug build"
                        } else {
                            "release build"
                        }
                    ),
                ),
                row("Qt", ctx.qt_version.clone()),
                row(
                    "Package",
                    env("FLATPAK_ID")
                        .map(|id| format!("Flatpak sandbox · {id}"))
                        .unwrap_or_else(|| "not sandboxed".into()),
                ),
                row("Licence", "GPL-3.0-or-later · © 2026 Fatih Bucaklioglu"),
            ],
        },
        Section {
            title: "System".into(),
            rows: vec![
                row("Operating system", os_name()),
                row("Gaming Mode", if gaming { "yes (gamescope)" } else { "no (desktop)" }),
                row(
                    "Steam Deck",
                    if env("SteamDeck").as_deref() == Some("1") {
                        "yes"
                    } else {
                        "not detected"
                    },
                ),
                row("Window", format!("{} × {}", ctx.window.0, ctx.window.1)),
                row("Language", env("LANG").unwrap_or_else(|| "not set".into())),
            ],
        },
        Section {
            title: "Store".into(),
            rows: vec![
                row("Mode", ctx.backend_mode.clone()),
                row(
                    "Status",
                    if ctx.backend_status.is_empty() {
                        "test run".into()
                    } else {
                        ctx.backend_status.clone()
                    },
                ),
                row("Install scope", "user only (flatpak --user, no password)"),
                row("Proton tools folder", "~/.steam/root/compatibilitytools.d"),
            ],
        },
        Section {
            title: "Controller".into(),
            rows: if ctx.controllers.is_empty() {
                vec![row("Connected", "none found (keyboard and touch still work)")]
            } else {
                ctx.controllers
                    .iter()
                    .map(|c| {
                        let layout = if crate::input::uses_position_layout(c) {
                            " · PlayStation layout"
                        } else {
                            ""
                        };
                        row("Connected", format!("{c}{layout}"))
                    })
                    .collect()
            },
        },
    ];
    let mut files = Vec::new();
    match &ctx.settings_path {
        Some(p) => files.push(row("Settings", size_of(p))),
        None => files.push(row("Settings", "not saved (test run)")),
    }
    match &ctx.history_path {
        Some(p) => {
            let n = crate::history::History::open(Some(p))
                .map(|h| h.recent(100_000).len())
                .unwrap_or(0);
            files.push(row("History database", format!("{} · {n} operations", size_of(p))));
        }
        None => files.push(row("History database", "in memory (test run)")),
    }
    if let Some(d) = crate::logging::log_dir() {
        files.push(row("Log", size_of(&d.join("arxis-deck.log"))));
    }
    out.push(Section {
        title: "Files".into(),
        rows: files,
    });
    out
}

/// The sections as JSON for the UI, with the home folder shown as `~`.
pub fn to_json(sections: &[Section]) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    redact(&serde_json::to_string(sections).unwrap_or_else(|_| "[]".into()), &home)
}

/// A plain-text report: all sections plus the last 200 log lines, home folder redacted.
pub fn report(sections: &[Section]) -> String {
    let mut s = String::from("Arxis Deck diagnostics report\n=============================\n\n");
    for sec in sections {
        s.push_str(&format!("[{}]\n", sec.title));
        for r in &sec.rows {
            s.push_str(&format!("{}: {}\n", r.key, r.value));
        }
        s.push('\n');
    }
    s.push_str("[Recent log]\n");
    for line in log_tail(200) {
        s.push_str(&line);
        s.push('\n');
    }
    redact(&s, &std::env::var("HOME").unwrap_or_default())
}

/// Writes the report into `dir` as `arxis-deck-diagnostics-<unix time>.txt`; returns its path.
pub fn export(sections: &[Section], dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = dir.join(format!("arxis-deck-diagnostics-{stamp}.txt"));
    std::fs::write(&path, report(sections)).map_err(|e| e.to_string())?;
    tracing::info!(path = %path.display(), "diagnostics exported");
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context {
            backend_mode: "simulated".into(),
            backend_status: "test".into(),
            controllers: vec!["Microsoft X-Box 360 pad 0".into()],
            window: (1280, 800),
            qt_version: "6.11.2".into(),
            history_path: None,
            settings_path: None,
        }
    }

    #[test]
    fn sections_cover_the_basics() {
        let s = collect(&ctx());
        let titles: Vec<&str> = s.iter().map(|x| x.title.as_str()).collect();
        assert_eq!(titles, ["Arxis Deck", "System", "Store", "Controller", "Files"]);
        assert!(to_json(&s).contains("Microsoft X-Box 360 pad 0"));
    }

    #[test]
    fn export_writes_a_redacted_report() {
        let home = std::env::var("HOME").unwrap_or_default();
        let mut c = ctx();
        c.settings_path = Some(PathBuf::from(format!("{home}/.config/arxis-deck/config.toml")));
        let dir = std::env::temp_dir().join(format!("arxis-diag-test-{}", std::process::id()));
        let p = export(&collect(&c), &dir).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        assert!(text.contains("[Store]") && text.contains("~/.config/arxis-deck/config.toml"));
        if home.len() > 1 {
            assert!(!text.contains(&home), "home folder must be redacted");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
