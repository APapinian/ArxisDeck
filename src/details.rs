// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! What the user sees before installing (ROADMAP.md M2 acceptance, PROJECT.md §2.8, §5.1):
//! version, licence, download and installed size from `flatpak remote-info`, and the app's
//! sandbox permissions from `flatpak remote-info --show-metadata`, in plain language.
//!
//! Each permission gets a level: "high" (needs attention: e.g. all files, whole home folder,
//! running commands outside the sandbox), "medium" (e.g. internet, drives, sound), "low"
//! (e.g. showing windows, the graphics card).

use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct Permission {
    pub label: String,
    pub detail: String,
    pub level: &'static str,
}

#[derive(Serialize, Debug, Default)]
pub struct AppDetails {
    pub id: String,
    pub version: String,
    pub license: String,
    pub download: String,
    pub installed: String,
    pub runtime: String,
    pub date: String,
    pub permissions: Vec<Permission>,
    /// Permissions with level "high".
    pub attention: usize,
    /// Empty when the details were read.
    pub error: String,
}

fn p(label: &str, detail: &str, level: &'static str) -> Permission {
    Permission {
        label: label.into(),
        detail: detail.into(),
        level,
    }
}

/// "Key: value" lines of `flatpak remote-info` (LC_ALL=C).
pub fn parse_info(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .filter(|(k, _)| !k.is_empty() && !k.contains(' '))
        .collect()
}

/// The `[Section]` → `key=value` map of a Flatpak metadata file.
fn sections(meta: &str) -> BTreeMap<String, Vec<(String, String)>> {
    let mut out: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut cur = String::new();
    for line in meta.lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            cur = line[1..line.len() - 1].to_string();
        } else if let Some((k, v)) = line.split_once('=') {
            out.entry(cur.clone())
                .or_default()
                .push((k.trim().into(), v.trim().into()));
        }
    }
    out
}

fn list(v: &str) -> impl Iterator<Item = &str> {
    v.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('!'))
}

fn filesystem(entry: &str) -> Permission {
    let (path, mode) = match entry.rsplit_once(':') {
        Some((p, m)) if matches!(m, "ro" | "rw" | "create") => (p, m),
        _ => (entry, "rw"),
    };
    let ro = mode == "ro";
    let rw = |high: &'static str| if ro { "medium" } else { high };
    let access = if ro { " (read only)" } else { "" };
    match path {
        "host" => p(&format!("All files on the device{access}"), entry, rw("high")),
        "host-os" | "host-etc" => p("System files", entry, "medium"),
        "home" | "~" => p(&format!("Your whole home folder{access}"), entry, rw("high")),
        "xdg-download" => p(&format!("Downloads folder{access}"), entry, "medium"),
        "xdg-documents" => p(&format!("Documents folder{access}"), entry, "medium"),
        "xdg-desktop" => p(&format!("Desktop folder{access}"), entry, "medium"),
        "xdg-music" => p(&format!("Music folder{access}"), entry, "medium"),
        "xdg-pictures" => p(&format!("Pictures folder{access}"), entry, "medium"),
        "xdg-videos" => p(&format!("Videos folder{access}"), entry, "medium"),
        "/media" | "/run/media" | "/mnt" => p("Removable drives (SD card, USB)", entry, "medium"),
        _ if path.starts_with("xdg-run/")
            || path.starts_with("xdg-config/")
            || path.starts_with("xdg-data/")
            || path.starts_with("xdg-cache/") =>
        {
            p("App-specific folder", entry, "low")
        }
        _ => p(&format!("Folder {path}{access}"), entry, "medium"),
    }
}

/// Plain-language permissions from Flatpak metadata, most important first.
pub fn parse_permissions(meta: &str) -> Vec<Permission> {
    let s = sections(meta);
    let mut out = Vec::new();
    if let Some(ctx) = s.get("Context") {
        for (k, v) in ctx {
            for item in list(v) {
                let perm = match (k.as_str(), item) {
                    ("shared", "network") => Some(p("Internet access", item, "medium")),
                    ("sockets", "x11" | "fallback-x11" | "wayland") => Some(p("Shows windows", "display", "low")),
                    ("sockets", "pulseaudio") => Some(p("Sound and microphone", item, "medium")),
                    ("sockets", "session-bus") => Some(p("All desktop services (session bus)", item, "high")),
                    ("sockets", "system-bus") => Some(p("All system services (system bus)", item, "high")),
                    ("sockets", "ssh-auth") => Some(p("Your SSH keys", item, "high")),
                    ("sockets", "gpg-agent") => Some(p("Your GPG keys", item, "high")),
                    ("sockets", "pcsc") => Some(p("Smart cards", item, "medium")),
                    ("sockets", "cups") => Some(p("Printing", item, "low")),
                    ("devices", "dri") => Some(p("Graphics card", item, "low")),
                    ("devices", "all") => Some(p("All devices: controllers, USB, cameras", item, "medium")),
                    ("devices", "input") => Some(p("Input devices", item, "low")),
                    ("devices", "kvm") => Some(p("Virtual machines", item, "medium")),
                    ("features", "bluetooth") => Some(p("Bluetooth", item, "medium")),
                    ("features", "devel") => Some(p("Can inspect other programs (devel)", item, "medium")),
                    ("filesystems", _) => Some(filesystem(item)),
                    _ => None,
                };
                if let Some(perm) = perm {
                    if !out.contains(&perm) {
                        out.push(perm);
                    }
                }
            }
        }
    }
    for (section, scope) in [("Session Bus Policy", "desktop"), ("System Bus Policy", "system")] {
        let Some(names) = s.get(section) else { continue };
        let mut others = 0;
        for (name, _) in names {
            match name.as_str() {
                "org.freedesktop.Flatpak" => out.push(p("Can run commands outside its sandbox", name, "high")),
                "org.freedesktop.secrets" => out.push(p("Saved passwords (keyring)", name, "medium")),
                "org.freedesktop.UDisks2" => out.push(p("Mount and manage drives", name, "medium")),
                _ => others += 1,
            }
        }
        if others > 0 {
            let level = if scope == "system" { "medium" } else { "low" };
            out.push(p(
                &format!(
                    "Talks to {others} {scope} service{}",
                    if others == 1 { "" } else { "s" }
                ),
                section,
                level,
            ));
        }
    }
    // Several app-specific folders are one line.
    let app_folders = out.iter().filter(|x| x.label == "App-specific folder").count();
    out.retain(|x| x.label != "App-specific folder");
    if app_folders > 0 {
        out.push(p(
            &format!(
                "{app_folders} app-specific folder{}",
                if app_folders == 1 { "" } else { "s" }
            ),
            "xdg-*",
            "low",
        ));
    }
    let rank = |l: &str| match l {
        "high" => 0,
        "medium" => 1,
        _ => 2,
    };
    out.sort_by_key(|x| rank(x.level));
    out
}

pub fn build(id: &str, info: &str, meta: &str) -> AppDetails {
    let f = parse_info(info);
    let get = |k: &str| f.get(k).cloned().unwrap_or_default();
    let permissions = parse_permissions(meta);
    AppDetails {
        id: id.into(),
        version: get("Version"),
        license: get("License"),
        download: get("Download"),
        installed: get("Installed"),
        runtime: get("Runtime"),
        date: get("Date").get(..10).unwrap_or("").to_string(),
        attention: permissions.iter().filter(|x| x.level == "high").count(),
        permissions,
        error: String::new(),
    }
}

/// Sample details for the simulated store (goes through the same parsers).
pub fn simulated(id: &str) -> AppDetails {
    build(
        id,
        "Version: 1.0 (sample)\nLicense: sample\nDownload: 12.0 MB\nInstalled: 40.0 MB\nRuntime: org.freedesktop.Platform/x86_64/25.08\nDate: 2026-11-01 10:00:00 +0000\n",
        "[Context]\nshared=network;ipc;\nsockets=wayland;fallback-x11;pulseaudio;\ndevices=dri;\nfilesystems=xdg-download;xdg-config/sample:create;\n",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Recorded from `LC_ALL=C flatpak remote-info --user flathub org.gnome.TextEditor` on 2026-11-06.
    const TEXT_EDITOR_INFO: &str = "Metin Düzenleyici - Metin dosyalarını düzenle

        ID: org.gnome.TextEditor
       Ref: app/org.gnome.TextEditor/x86_64/stable
      Arch: x86_64
    Branch: stable
   Version: 51.0
   License: GPL-3.0-or-later
Collection: org.flathub.Stable
  Download: 1.3 MB
 Installed: 2.4 MB
   Runtime: org.gnome.Platform/x86_64/51
       Sdk: org.gnome.Sdk/x86_64/51

    Commit: 76ecd1d6e9ecfa26f6939c54eec4e9db3491fce949c8808bdee9fb55758a1a91
      Date: 2026-09-17 20:09:37 +0000";

    const TEXT_EDITOR_META: &str = "[Application]
name=org.gnome.TextEditor

[Context]
shared=ipc;
sockets=fallback-x11;wayland;
devices=dri;
filesystems=xdg-run/gvfsd;host;xdg-run/gvfs;

[Session Bus Policy]
org.gtk.vfs.*=talk
org.freedesktop.FileManager1=talk
";

    /// The [Context] and bus sections of com.heroicgameslauncher.hgl, recorded 2026-11-06.
    const HEROIC_META: &str = "[Context]
shared=ipc;network;
sockets=pulseaudio;wayland;x11;
devices=all;
features=bluetooth;devel;multiarch;per-app-dev-shm;
filesystems=/run/media;~/.local/share/applications;/run/udev:ro;xdg-run/gamescope-0;~/.steam/steam;/mnt;~/Games/Heroic:create;xdg-documents;/media;xdg-desktop;xdg-config/MangoHud:ro;~/.steam;

[Session Bus Policy]
org.kde.StatusNotifierWatcher=talk
org.freedesktop.Notifications=talk

[System Bus Policy]
org.freedesktop.UPower=talk
org.freedesktop.UDisks2=talk
";

    #[test]
    fn text_editor_details() {
        let d = build("org.gnome.TextEditor", TEXT_EDITOR_INFO, TEXT_EDITOR_META);
        assert_eq!(d.version, "51.0");
        assert_eq!(d.license, "GPL-3.0-or-later");
        assert_eq!(d.download, "1.3 MB");
        assert_eq!(d.installed, "2.4 MB");
        assert_eq!(d.date, "2026-09-17");
        assert_eq!(d.permissions[0].label, "All files on the device", "high first");
        assert_eq!(d.attention, 1);
        let labels: Vec<&str> = d.permissions.iter().map(|x| x.label.as_str()).collect();
        assert!(labels.contains(&"Shows windows") && labels.contains(&"Graphics card"));
        assert!(labels.contains(&"2 app-specific folders"));
        assert!(labels.contains(&"Talks to 2 desktop services"));
        assert_eq!(
            labels.iter().filter(|l| **l == "Shows windows").count(),
            1,
            "x11 + wayland is one line"
        );
    }

    #[test]
    fn heroic_permissions() {
        let perms = parse_permissions(HEROIC_META);
        let has = |l: &str| perms.iter().any(|x| x.label == l);
        assert!(has("Internet access"));
        assert!(has("All devices: controllers, USB, cameras"));
        assert!(has("Removable drives (SD card, USB)"));
        assert!(has("Folder ~/.steam/steam"));
        assert!(has("Folder /run/udev (read only)"));
        assert!(has("Mount and manage drives"));
        assert!(has("Talks to 1 system service"));
        assert!(has("Bluetooth") && has("Can inspect other programs (devel)"));
        assert!(perms.iter().all(|x| x.level != "high"), "no whole-home or host access");
    }

    #[test]
    fn dangerous_permissions_are_high() {
        let perms =
            parse_permissions("[Context]\nfilesystems=home;\n[Session Bus Policy]\norg.freedesktop.Flatpak=talk\n");
        assert_eq!(perms.iter().filter(|x| x.level == "high").count(), 2);
        let ro = parse_permissions("[Context]\nfilesystems=home:ro;\n");
        assert_eq!(ro[0].label, "Your whole home folder (read only)");
        assert_eq!(ro[0].level, "medium");
    }

    #[test]
    fn simulated_details_use_the_parsers() {
        let d = simulated("sample.item1");
        assert_eq!(d.version, "1.0 (sample)");
        assert!(d.permissions.iter().any(|x| x.label == "Internet access"));
    }
}
