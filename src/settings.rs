// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! Settings persistence. QML owns the values; Rust loads the file once at start-up
//! (into the QML `savedSettings` property, as `key=value` lines) and writes `settingsBlob`
//! back when it changes.
//!
//! File: `$XDG_CONFIG_HOME/arxis-deck/config.toml` (or `~/.config/arxis-deck/config.toml`),
//! as in ARCHITECTURE.md §8. Values are plain TOML strings: `theme = "Arch Midnight"`.
//! A `settings.conf` from earlier test builds is read once if `config.toml` does not exist.

use crate::qt;
use std::fs;
use std::path::PathBuf;

/// How often (in ticks) QML is polled for changed settings.
pub const POLL_TICKS: u64 = 15;

pub fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("arxis-deck").join("config.toml"))
}

/// `key=value` lines (the QML format) → TOML.
pub fn to_toml(blob: &str) -> String {
    let mut out = String::from("# Arxis Deck settings (written by the app)\n");
    for line in blob.lines() {
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim();
            if !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                out.push_str(&format!(
                    "{k} = \"{}\"\n",
                    v.trim().replace('\\', "\\\\").replace('"', "\\\"")
                ));
            }
        }
    }
    out
}

/// TOML (`key = "value"`, comments ignored) or old `key=value` lines → `key=value` lines.
pub fn from_toml(text: &str) -> String {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim();
            let v = if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
                v[1..v.len() - 1].replace("\\\"", "\"").replace("\\\\", "\\")
            } else {
                v.to_string()
            };
            out.push(format!("{}={}", k.trim(), v));
        }
    }
    out.join("\n")
}

pub struct Store {
    path: Option<PathBuf>,
    last: Option<String>,
}

impl Store {
    /// `path: None` keeps settings in memory only.
    pub fn new(path: Option<PathBuf>) -> Store {
        Store { path, last: None }
    }

    pub fn load_into_ui(&self) {
        let Some(path) = self.path.as_ref() else { return };
        let text = fs::read_to_string(path)
            .ok()
            .or_else(|| fs::read_to_string(path.with_file_name("settings.conf")).ok());
        if let Some(text) = text {
            qt::set_property("savedSettings", &from_toml(&text));
        }
    }

    /// Settings → "X and Y buttons: Swapped".
    pub fn swap_xy(&self) -> bool {
        qt::property_bool("xySwap")
    }

    /// Writes the settings file if the UI's settings changed since the last call.
    /// Returns the current "delay,interval" repeat timing for the Navigator.
    pub fn sync(&mut self) -> Option<(u64, u64)> {
        let blob = qt::property("settingsBlob");
        match &self.last {
            None => self.last = Some(blob),
            Some(prev) if *prev != blob => {
                if let Some(path) = &self.path {
                    if let Some(dir) = path.parent() {
                        let _ = fs::create_dir_all(dir);
                    }
                    if let Err(e) = fs::write(path, to_toml(&blob)) {
                        tracing::warn!("could not save settings to {}: {e}", path.display());
                    }
                }
                self.last = Some(blob);
            }
            _ => {}
        }
        let timing = qt::property("repeatTiming");
        let (d, i) = timing.split_once(',')?;
        Some((d.trim().parse().ok()?, i.trim().parse().ok()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_round_trip() {
        let blob = "theme=Arch Midnight\nrepeat=Fast\nxy=Swapped";
        let toml = to_toml(blob);
        assert!(toml.contains("theme = \"Arch Midnight\""));
        assert_eq!(from_toml(&toml), blob);
        // Old settings.conf lines still load.
        assert_eq!(from_toml("theme=Nord Arctic\n"), "theme=Nord Arctic");
    }
}
