// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! AppImage provider (ROADMAP.md M3; PROJECT.md §5.2 "AppImage"; ARCHITECTURE.md §5.2).
//!
//! Rules from the documents, in order of the install:
//! 1. curated GitHub projects; the file is chosen by explainable rules (`choose`);
//! 2. download into a staging folder under the user's data area, never into place directly;
//! 3. check the size GitHub reports and the SHA-256 GitHub computed at upload (labelled as
//!    integrity, not publisher identity), and that the file is an ELF program;
//! 4. move it to `~/Applications/` **without** the executable bit — nothing runs automatically;
//! 5. only when the user confirms "Make runnable" does it get `chmod 755` and a start-menu entry
//!    (`~/.local/share/applications/arxis-deck-<slug>.desktop`) so Steam can add it;
//! 6. every file Arxis Deck creates is listed in a manifest; uninstall removes only those.

use crate::github::{Asset, Release};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const PREFIX: &str = "appimage:";

/// (GitHub repository, name, category, summary). Checked on 2026-11-06: each publishes an x86_64
/// AppImage with a GitHub SHA-256 digest in its latest release.
pub const CURATED: &[(&str, &str, &str, &str)] = &[
    (
        "stenzek/duckstation",
        "DuckStation",
        "Emulator",
        "PlayStation 1 emulator",
    ),
    ("Vita3K/Vita3K", "Vita3K", "Emulator", "PlayStation Vita emulator"),
    ("xemu-project/xemu", "xemu", "Emulator", "Original Xbox emulator"),
];

/// A catalogue card for an AppImage, in the shape the QML catalogue uses.
#[derive(Serialize, Debug, PartialEq, Clone)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub category: String,
    pub summary: String,
    pub icon: String,
    pub source: String,
    pub url: String,
    /// SHA-256 hex from GitHub, or "" (shown as not verified).
    pub sum: String,
    pub size: u64,
    pub file: String,
    pub version: String,
    pub repo: String,
}

/// `owner/repo` → `appimage:owner.repo`.
pub fn id_for(repo: &str) -> String {
    format!("{PREFIX}{}", repo.replace('/', "."))
}

/// AppImage file names: letters, digits, `._+-`, ending in `.AppImage`, no leading dot/dash.
pub fn valid_file_name(n: &str) -> bool {
    n.ends_with(".AppImage")
        && n.len() <= 200
        && !n.starts_with(['.', '-'])
        && n.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'))
}

/// Rules, in order: skip drafts/prereleases (unless the project only ships a rolling
/// "continuous"/"latest" build); the asset must end in `.AppImage`; leave out ARM builds, debug
/// builds and legacy `SSE2` builds; prefer names that say x86_64/x64/amd64/linux64; the link must
/// be a GitHub release download and the name a safe file name.
pub fn choose(r: &Release) -> Option<&Asset> {
    if r.draft {
        return None;
    }
    let lower = |a: &Asset| a.name.to_lowercase();
    let candidates: Vec<&Asset> = r
        .assets
        .iter()
        .filter(|a| a.name.ends_with(".AppImage") && valid_file_name(&a.name))
        .filter(|a| {
            let n = lower(a);
            !["arm64", "aarch64", "armhf", "armv7", "-dbg", "debug", "sse2"]
                .iter()
                .any(|w| n.contains(w))
        })
        .filter(|a| crate::store::valid_release_url(&a.browser_download_url))
        .collect();
    let says_x86 = |a: &&Asset| {
        ["x86_64", "x64", "amd64", "linux64"]
            .iter()
            .any(|w| lower(a).contains(w))
    };
    candidates
        .iter()
        .copied()
        .find(|a| says_x86(a))
        .or(candidates.first().copied())
}

/// The hex part of GitHub's `sha256:<hex>` digest, if it is well formed.
pub fn sha256_of(a: &Asset) -> String {
    a.digest
        .as_deref()
        .and_then(|d| d.strip_prefix("sha256:"))
        .filter(|h| h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or("")
        .to_string()
}

/// Builds the card for one curated project from its `releases/latest` JSON.
pub fn item_from_release(json: &str, repo: &str, name: &str, category: &str, summary: &str) -> Result<Item, String> {
    let r: Release = serde_json::from_str(json).map_err(|e| format!("unexpected GitHub reply: {e}"))?;
    let a = choose(&r).ok_or_else(|| format!("{repo}: no x86_64 AppImage in the latest release"))?;
    let date = r.published_at.as_deref().unwrap_or("").get(..10).unwrap_or("");
    Ok(Item {
        id: id_for(repo),
        name: name.into(),
        category: category.into(),
        summary: format!(
            "{summary} · AppImage {} from github.com/{repo} · released {date} · {} MB",
            r.tag_name,
            (a.size + 524_288) / 1_048_576
        ),
        icon: String::new(),
        source: "appimage".into(),
        url: a.browser_download_url.clone(),
        sum: sha256_of(a),
        size: a.size,
        file: a.name.clone(),
        version: r.tag_name.clone(),
        repo: repo.into(),
    })
}

// ---- Manifest of files Arxis Deck manages ----

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub repo: String,
    /// `~/Applications/<file>` as an absolute path.
    pub path: String,
    pub sha256: String,
    /// Made executable and given a start-menu entry by the user.
    pub registered: bool,
    /// The `.desktop` file Arxis Deck wrote, if registered.
    pub desktop: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Manifest {
    pub version: u32,
    pub entries: Vec<Entry>,
}

/// `~/.local/share/arxis-deck/appimages.json` (inside the sandbox: the app's own data folder).
pub fn manifest_path() -> Option<PathBuf> {
    crate::logging::log_dir().and_then(|d| d.parent().map(|p| p.join("appimages.json")))
}

impl Manifest {
    pub fn load(path: Option<&Path>) -> Manifest {
        path.and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(Manifest {
                version: 1,
                entries: Vec::new(),
            })
    }

    pub fn save(&self, path: Option<&Path>) -> Result<(), String> {
        let Some(p) = path else { return Ok(()) };
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, p).map_err(|e| e.to_string())
    }

    pub fn get(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }
}

// ---- Host scripts (run with `sh -c`, arguments as $1… never interpolated) ----

/// $1 URL, $2 SHA-256 ("" = none), $3 file name, $4 KB needed, $5 expected size in bytes.
/// Prints the final path on success. The file is left without the executable bit.
pub const INSTALL_SH: &str = r#"set -e
stage="$HOME/.local/share/arxis-deck/staging"
dest="$HOME/Applications"
mkdir -p "$stage" "$dest"
[ ! -e "$dest/$3" ] || { echo "error: $3 is already in ~/Applications"; exit 1; }
if [ "${4:-0}" -gt 0 ]; then
  avail=$(df -Pk "$stage" | awk 'NR == 2 { print $4 }')
  [ "$avail" -ge "$4" ] || { echo "error: not enough free space: need $(( $4 / 1024 )) MB, have $(( avail / 1024 )) MB"; exit 1; }
fi
part="$stage/$3.part"
trap 'rm -f "$part"' EXIT
curl -fL --progress-bar -o "$part" "$1"
if [ "${5:-0}" -gt 0 ]; then
  [ "$(stat -c %s "$part")" = "$5" ] || { echo "error: the download has the wrong size"; exit 1; }
fi
if [ -n "$2" ]; then
  [ "$(sha256sum "$part" | cut -d' ' -f1)" = "$2" ] || { echo "error: checksum does not match"; exit 1; }
fi
[ "$(head -c 4 "$part" | od -An -tx1 | tr -d ' \n')" = "7f454c46" ] || { echo "error: the download is not an AppImage program"; exit 1; }
chmod 0644 "$part"
mv "$part" "$dest/$3"
trap - EXIT
echo "path:$dest/$3"
"#;

/// $1 AppImage path (inside ~/Applications), $2 .desktop file name, $3 display name.
/// Makes it executable and writes a start-menu entry; prints the .desktop path.
pub const REGISTER_SH: &str = r#"set -e
case "$1" in "$HOME/Applications/"*.AppImage) ;; *) echo "error: not a managed AppImage"; exit 1 ;; esac
[ -f "$1" ] || { echo "error: the file is missing"; exit 1; }
apps="$HOME/.local/share/applications"
mkdir -p "$apps"
chmod 0755 "$1"
exec_line="\"$1\""
command -v fusermount >/dev/null 2>&1 || command -v fusermount3 >/dev/null 2>&1 || exec_line="env APPIMAGE_EXTRACT_AND_RUN=1 \"$1\""
cat > "$apps/$2" <<EOF
[Desktop Entry]
Type=Application
Name=$3
Comment=AppImage installed by Arxis Deck
Exec=$exec_line
Terminal=false
Categories=Game;
EOF
echo "desktop:$apps/$2"
"#;

/// Each argument is a file Arxis Deck created; only ~/Applications/*.AppImage and
/// ~/.local/share/applications/arxis-deck-*.desktop are accepted, anything else is refused.
pub const REMOVE_SH: &str = r#"set -e
for f in "$@"; do
  case "$f" in
    "$HOME/Applications/"*.AppImage|"$HOME/.local/share/applications/arxis-deck-"*.desktop) ;;
    *) echo "error: refusing to remove $f"; exit 1 ;;
  esac
  case "$f" in *..*) echo "error: refusing to remove $f"; exit 1 ;; esac
done
for f in "$@"; do rm -f -- "$f"; done
"#;

/// `arxis-deck-<owner>.<repo>.desktop` for an AppImage ID.
pub fn desktop_name(id: &str) -> String {
    format!("arxis-deck-{}.desktop", id.trim_start_matches(PREFIX))
}

/// Two simulated releases' worth of AppImage cards (no network).
pub fn simulated_items() -> Vec<Item> {
    vec![Item {
        id: id_for("sample/emu"),
        name: "Sample AppImage".into(),
        category: "Tool".into(),
        summary: "Sample emulator · AppImage v1.0 from github.com/sample/emu · 50 MB".into(),
        icon: String::new(),
        source: "appimage".into(),
        url: "https://github.com/sample/emu/releases/download/v1.0/Sample-x86_64.AppImage".into(),
        sum: "0".repeat(64),
        size: 50 * 1_048_576,
        file: "Sample-x86_64.AppImage".into(),
        version: "v1.0".into(),
        repo: "sample/emu".into(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn release(names: &[&str]) -> String {
        let assets: Vec<String> = names
            .iter()
            .map(|n| {
                format!(
                    r#"{{"name": "{n}", "size": 94519584, "browser_download_url": "https://github.com/a/b/releases/download/latest/{n}", "digest": "sha256:{}"}}"#,
                    "4d".repeat(32)
                )
            })
            .collect();
        format!(
            r#"{{"tag_name": "latest", "published_at": "2026-10-25T02:59:23Z", "prerelease": false, "assets": [{}]}}"#,
            assets.join(", ")
        )
    }

    #[test]
    fn picks_the_right_file_from_real_release_names() {
        // DuckStation, 2026-10-25
        let j = release(&[
            "DuckStation-arm64.AppImage",
            "DuckStation-armhf.AppImage",
            "DuckStation-x64-SSE2.AppImage",
            "DuckStation-x64.AppImage",
        ]);
        let it = item_from_release(&j, "stenzek/duckstation", "DuckStation", "Emulator", "PS1").unwrap();
        assert_eq!(it.file, "DuckStation-x64.AppImage");
        assert_eq!(it.sum.len(), 64);
        assert_eq!(it.id, "appimage:stenzek.duckstation");
        // xemu, 2026-10-21
        let j = release(&[
            "xemu-0.8.139-aarch64.AppImage",
            "xemu-0.8.139-dbg-aarch64.AppImage",
            "xemu-0.8.139-dbg-x86_64.AppImage",
            "xemu-0.8.139-x86_64.AppImage",
        ]);
        assert_eq!(
            item_from_release(&j, "xemu-project/xemu", "xemu", "Emulator", "Xbox")
                .unwrap()
                .file,
            "xemu-0.8.139-x86_64.AppImage"
        );
        // Vita3K: zsync files are not AppImages
        let j = release(&[
            "Vita3K-aarch64.AppImage",
            "Vita3K-x86_64.AppImage.zsync",
            "Vita3K-x86_64.AppImage",
        ]);
        assert_eq!(
            item_from_release(&j, "Vita3K/Vita3K", "Vita3K", "Emulator", "Vita")
                .unwrap()
                .file,
            "Vita3K-x86_64.AppImage"
        );
        // no x86 build
        assert!(item_from_release(&release(&["Only-arm64.AppImage"]), "a/b", "B", "Tool", "").is_err());
    }

    #[test]
    fn file_names_and_digests_are_checked() {
        assert!(valid_file_name("DuckStation-x64.AppImage"));
        assert!(!valid_file_name("../x.AppImage"));
        assert!(!valid_file_name("x.AppImage; rm"));
        let a = Asset {
            name: "x.AppImage".into(),
            size: 1,
            browser_download_url: String::new(),
            digest: Some("sha256:zz".into()),
        };
        assert_eq!(sha256_of(&a), "", "malformed digest → not verified");
    }

    #[test]
    fn manifest_round_trip() {
        let p = std::env::temp_dir().join(format!("arxis-appimage-manifest-{}.json", std::process::id()));
        let mut m = Manifest::load(Some(&p));
        assert!(m.entries.is_empty());
        m.entries.push(Entry {
            id: "appimage:a.b".into(),
            name: "B".into(),
            version: "v1".into(),
            repo: "a/b".into(),
            path: "/home/u/Applications/B.AppImage".into(),
            sha256: String::new(),
            registered: false,
            desktop: None,
        });
        m.save(Some(&p)).unwrap();
        assert_eq!(Manifest::load(Some(&p)).get("appimage:a.b").unwrap().name, "B");
        let _ = std::fs::remove_file(&p);
    }

    /// Runs the real scripts in a temporary HOME with file:// downloads.
    #[test]
    fn install_register_remove_scripts() {
        let root = std::env::temp_dir().join(format!("arxis-appimage-test-{}", std::process::id()));
        let home = root.join("home");
        std::fs::create_dir_all(&home).unwrap();
        // A tiny "AppImage": ELF magic + padding.
        let mut elf = vec![0x7f, b'E', b'L', b'F'];
        elf.extend_from_slice(&[0u8; 60]);
        std::fs::write(root.join("good.AppImage"), &elf).unwrap();
        std::fs::write(root.join("text.AppImage"), b"#!/bin/sh\necho hi\n").unwrap();
        let sha = |f: &str| {
            let o = Command::new("sha256sum").arg(root.join(f)).output().unwrap();
            String::from_utf8_lossy(&o.stdout)
                .split(' ')
                .next()
                .unwrap()
                .to_string()
        };
        let url = |f: &str| format!("file://{}", root.join(f).display());
        let sh = |script: &str, args: &[&str]| {
            Command::new("sh")
                .arg("-c")
                .arg(script)
                .arg("sh")
                .args(args)
                .env("HOME", &home)
                .output()
                .unwrap()
        };
        let size = elf.len().to_string();

        // wrong checksum, wrong size and a non-program are refused; nothing lands in ~/Applications
        let bad = sh(
            INSTALL_SH,
            &[&url("good.AppImage"), &"0".repeat(64), "App.AppImage", "0", &size],
        );
        assert!(String::from_utf8_lossy(&bad.stdout).contains("checksum does not match"));
        let bad = sh(INSTALL_SH, &[&url("good.AppImage"), "", "App.AppImage", "0", "999"]);
        assert!(String::from_utf8_lossy(&bad.stdout).contains("wrong size"));
        let bad = sh(INSTALL_SH, &[&url("text.AppImage"), "", "App.AppImage", "0", "0"]);
        assert!(String::from_utf8_lossy(&bad.stdout).contains("not an AppImage"));
        assert!(!home.join("Applications/App.AppImage").exists());
        assert!(
            !home.join(".local/share/arxis-deck/staging/App.AppImage.part").exists(),
            "staging is cleaned up"
        );

        // good file: saved, NOT executable
        let ok = sh(
            INSTALL_SH,
            &[&url("good.AppImage"), &sha("good.AppImage"), "App.AppImage", "0", &size],
        );
        assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stdout));
        let app = home.join("Applications/App.AppImage");
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&app).unwrap().permissions().mode() & 0o111,
            0,
            "not runnable before the user agrees"
        );
        assert!(
            !sh(INSTALL_SH, &[&url("good.AppImage"), "", "App.AppImage", "0", "0"])
                .status
                .success(),
            "no overwrite"
        );

        // register: executable + desktop entry
        let app_s = app.display().to_string();
        let reg = sh(REGISTER_SH, &[&app_s, "arxis-deck-a.b.desktop", "App"]);
        assert!(reg.status.success(), "{}", String::from_utf8_lossy(&reg.stdout));
        assert_ne!(std::fs::metadata(&app).unwrap().permissions().mode() & 0o111, 0);
        let desktop = home.join(".local/share/applications/arxis-deck-a.b.desktop");
        assert!(std::fs::read_to_string(&desktop).unwrap().contains("Name=App"));
        assert!(
            !sh(REGISTER_SH, &["/etc/passwd", "x.desktop", "X"]).status.success(),
            "only managed paths"
        );

        // remove: refuses anything outside the managed places, then removes exactly the listed files
        let other = home.join("Applications/keep.txt");
        std::fs::write(&other, "user file").unwrap();
        assert!(!sh(REMOVE_SH, &[&other.display().to_string()]).status.success());
        assert!(!sh(
            REMOVE_SH,
            &[&format!("{}/Applications/../.bashrc.AppImage", home.display())]
        )
        .status
        .success());
        let rm = sh(REMOVE_SH, &[&app_s, &desktop.display().to_string()]);
        assert!(rm.status.success());
        assert!(!app.exists() && !desktop.exists());
        assert!(other.exists(), "unrelated files stay");

        let _ = std::fs::remove_dir_all(&root);
    }
}
