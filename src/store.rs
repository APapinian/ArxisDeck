// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! Store backend: installs, removes and searches apps.
//!
//! * `Flatpak`: real Flathub apps, installed for the current user (`flatpak --user`), so no
//!   password is needed and SteamOS updates keep them. Inside a Flatpak sandbox the host's
//!   `flatpak` is reached through `flatpak-spawn --host`.
//! * `Simulated`: same commands and events, nothing touches the system. Used by the
//!   automated checks and when `flatpak` is missing.
//!
//! The UI sends [`Cmd`]s; the backend answers with [`Event`]s, delivered on the GUI thread.

use crate::history::History;
use crate::json;
use std::collections::{HashSet, VecDeque};
use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

pub const FLATHUB_REPO: &str = "https://dl.flathub.org/repo/flathub.flatpakrepo";
/// Proton-GE releases (newest first) on GitHub.
pub const GE_PROTON_RELEASES: &str =
    "https://api.github.com/repos/GloriousEggroll/proton-ge-custom/releases?per_page=6";
/// IDs of Steam compatibility tools are "compat:<folder name>".
pub const COMPAT_PREFIX: &str = "compat:";

/// Sets $d to Steam's compatibility tools folder (SteamOS: ~/.steam/root links to ~/.local/share/Steam).
const COMPAT_DIR_SH: &str = r#"d="$HOME/.steam/root/compatibilitytools.d"
[ -d "$HOME/.steam/root" ] || d="$HOME/.local/share/Steam/compatibilitytools.d""#;

/// $1 = .tar.gz URL, $2 = .sha512sum URL (may be empty), $3 = folder the archive must create.
/// Refuses if that folder exists; downloads into a temporary folder that is always cleaned up,
/// checks the checksum, refuses archives with entries outside "$3/" or containing "..", unpacks.
const COMPAT_INSTALL_SH: &str = r#"set -e
mkdir -p "$d"
# Never overwrite a version the user already has (PROJECT.md §5.2, gaming tools).
[ ! -e "$d/$3" ] || { echo "error: $3 is already installed"; exit 1; }
# Free-space preflight: $4 = KB needed (download + unpacked), 0 = unknown.
if [ "${4:-0}" -gt 0 ]; then
  avail=$(df -Pk "$d" | awk 'NR == 2 { print $4 }')
  [ "$avail" -ge "$4" ] || { echo "error: not enough free space: need $(( $4 / 1024 )) MB, have $(( avail / 1024 )) MB"; exit 1; }
fi
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
curl -fL --progress-bar -o "$tmp/tool.tar.gz" "$1"
if [ -n "$2" ]; then
  curl -fsSL -o "$tmp/tool.sha512sum" "$2"
  want=$(cut -d' ' -f1 "$tmp/tool.sha512sum")
  have=$(sha512sum "$tmp/tool.tar.gz" | cut -d' ' -f1)
  [ "$want" = "$have" ] || { echo "error: checksum does not match"; exit 1; }
fi
echo "Checking archive"
bad=$(tar -tzf "$tmp/tool.tar.gz" | awk -v n="$3" 'BEGIN { p = n "/" }
  $0 != n && $0 != p && index($0, p) != 1 { print; exit }
  $0 ~ /(^|\/)\.\.(\/|$)/ { print; exit }')
[ -z "$bad" ] || { echo "error: unexpected path in archive: $bad"; exit 1; }
echo "Unpacking"
tar -xzf "$tmp/tool.tar.gz" -C "$d" --no-same-owner
[ -d "$d/$3" ] || { echo "error: the download did not contain $3"; exit 1; }
"#;

/// $1 = folder name inside $d (already validated: no slashes, no dot-dot).
const COMPAT_REMOVE_SH: &str = r#"[ -d "$d/$1" ] || { echo "not installed" >&2; exit 1; }
rm -rf -- "$d/$1"
"#;

/// A compatibility tool folder name like "GE-Proton11-12": no slashes, no leading dot or dash.
pub fn valid_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with(['.', '-'])
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+'))
}

/// Only GitHub release downloads are fetched.
pub fn valid_release_url(url: &str) -> bool {
    url.starts_with("https://github.com/")
        && url.contains("/releases/download/")
        && url
            .chars()
            .all(|c| c.is_ascii_graphic() && !matches!(c, '"' | '\'' | '`' | '\\' | '$'))
}

/// Two fake Proton-GE releases in GitHub's format, for the simulated store.
fn sim_releases_json() -> String {
    let rel = |tag: &str, date: &str| {
        let base = format!("https://github.com/GloriousEggroll/proton-ge-custom/releases/download/{tag}/{tag}");
        format!(
            r#"{{"tag_name": "{tag}", "published_at": "{date}", "prerelease": false, "assets": [{{"name": "{tag}.tar.gz", "size": 500000000, "browser_download_url": "{base}.tar.gz"}}, {{"name": "{tag}.sha512sum", "size": 160, "browser_download_url": "{base}.sha512sum"}}]}}"#
        )
    };
    format!(
        "[{}, {}]",
        rel("GE-Proton10-25", "2026-01-20T00:00:00Z"),
        rel("GE-Proton10-24", "2026-01-06T00:00:00Z")
    )
}

/// Simulated install speed: this fraction every SIM_STEP_MS.
pub const SIM_STEP: f32 = 0.05;
pub const SIM_STEP_MS: u64 = 100;

/// Steam Deck starter list: (Flathub app ID, name, category, summary).
/// Every ID was found with `flatpak search` on 2026-11-05.
pub const CURATED: &[(&str, &str, &str, &str)] = &[
    (
        "net.davidotek.pupgui2",
        "ProtonUp-Qt",
        "Compatibility",
        "Install and manage Proton-GE and other compatibility tools",
    ),
    (
        "com.vysp3r.ProtonPlus",
        "ProtonPlus",
        "Compatibility",
        "Manage Proton, Wine, DXVK and VKD3D versions",
    ),
    (
        "com.github.Matoking.protontricks",
        "Protontricks",
        "Compatibility",
        "Apply Winetricks fixes to Proton games",
    ),
    (
        "com.usebottles.bottles",
        "Bottles",
        "Compatibility",
        "Run Windows software in managed Wine prefixes",
    ),
    (
        "com.heroicgameslauncher.hgl",
        "Heroic Games Launcher",
        "Launcher",
        "Play your Epic, GOG and Amazon games",
    ),
    (
        "net.lutris.Lutris",
        "Lutris",
        "Launcher",
        "One library for games from many sources",
    ),
    (
        "io.github.philipk.boilr",
        "BoilR",
        "Tool",
        "Add games from other launchers to your Steam library",
    ),
    (
        "com.steamgriddb.steam-rom-manager",
        "Steam ROM Manager",
        "Tool",
        "Add emulated games to Steam with artwork",
    ),
    (
        "com.github.tchx84.Flatseal",
        "Flatseal",
        "Tool",
        "Review and change Flatpak app permissions",
    ),
    (
        "org.libretro.RetroArch",
        "RetroArch",
        "Emulator",
        "Front end for many emulators and game engines",
    ),
    (
        "org.DolphinEmu.dolphin-emu",
        "Dolphin",
        "Emulator",
        "GameCube and Wii emulator",
    ),
    ("net.pcsx2.PCSX2", "PCSX2", "Emulator", "PlayStation 2 emulator"),
    (
        "org.ppsspp.PPSSPP",
        "PPSSPP",
        "Emulator",
        "PlayStation Portable emulator",
    ),
    ("net.rpcs3.RPCS3", "RPCS3", "Emulator", "PlayStation 3 emulator"),
    ("info.cemu.Cemu", "Cemu", "Emulator", "Wii U emulator"),
    ("net.shadps4.shadPS4", "shadPS4", "Emulator", "PlayStation 4 emulator"),
    ("org.mozilla.firefox", "Firefox", "Utility", "Web browser"),
    (
        "org.videolan.VLC",
        "VLC",
        "Utility",
        "Plays almost any video or audio file",
    ),
    ("com.discordapp.Discord", "Discord", "Utility", "Voice and text chat"),
    (
        "com.obsproject.Studio",
        "OBS Studio",
        "Utility",
        "Record and stream gameplay",
    ),
];

/// The app's Flathub icon from the appstream data flatpak keeps on disk (user copy first,
/// then the system copy SteamOS keeps up to date), as a file:// URL; "" if there is none.
pub fn icon_url(id: &str) -> String {
    if !valid_app_id(id) {
        return String::new();
    }
    let arch = std::env::consts::ARCH;
    let mut bases = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        bases.push(std::path::PathBuf::from(home).join(".local/share/flatpak"));
    }
    bases.push(std::path::PathBuf::from("/var/lib/flatpak"));
    for base in bases {
        for size in ["128x128", "64x64"] {
            let p = base.join(format!("appstream/flathub/{arch}/active/icons/{size}/{id}.png"));
            if p.is_file() {
                return format!("file://{}", p.display());
            }
        }
    }
    String::new()
}

pub fn curated_json() -> String {
    let items: Vec<String> = CURATED
        .iter()
        .map(|(id, name, cat, sum)| {
            json::object(&[
                ("id", id),
                ("name", name),
                ("category", cat),
                ("summary", sum),
                ("icon", &icon_url(id)),
            ])
        })
        .collect();
    json::array(&items)
}

pub enum Cmd {
    /// Queue an install job (one at a time, in order).
    Install {
        id: String,
        name: String,
        job: Job,
    },
    CheckUpdates,
    Cancel(String),
    Uninstall(String),
    Search(String),
    Launch(String),
    /// Version, sizes and permissions of a Flathub app, before installing it.
    Details(String),
    /// Re-check whether Flathub can be reached now.
    CheckNetwork,
    /// Simulated store only: pretend to be offline (true) or online (false), for tests.
    SimOffline(bool),
    /// AppImage: make it executable and add a start-menu entry (after the user confirmed).
    Register(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Job {
    /// Install a Flathub app, or with `update` bring an installed one up to date.
    Flatpak { update: bool },
    /// Download a Steam compatibility tool (e.g. Proton-GE) from GitHub releases, check its
    /// SHA-512 when a checksum file is given, and unpack it into compatibilitytools.d.
    Compat { url: String, sum: String, need_kb: u64 },
    /// Download an AppImage into ~/Applications (see `crate::appimage`); not made executable.
    AppImage {
        url: String,
        sha256: String,
        file: String,
        size: u64,
        version: String,
        repo: String,
        need_kb: u64,
    },
}

pub enum Event {
    Progress(String, f32),
    /// "ok", "cancelled" or an error message.
    Done(String, String),
    /// JSON array of {id, name, version}.
    Installed(String),
    /// "ok" or an error message.
    Removed(String, String),
    /// JSON array of {id, name, category, summary, icon}.
    CatalogAdd(String),
    /// "ok" or an error message.
    Launched(String, String),
    /// JSON array of {id, version}: installed apps with an update on Flathub.
    Updates(String),
    /// JSON array of recent operations (see `history::Operation`), newest first.
    History(String),
    /// (app ID, JSON of `details::AppDetails`).
    Details(String, String),
    /// ("online" | "offline", message).
    Network(String, String),
    /// (query, "ok:<count>" or an error message).
    SearchDone(String, String),
    /// A message for the user (e.g. the Proton-GE list could not be loaded).
    Notice(String),
    /// (AppImage ID, "ok" or an error) after "make runnable".
    Registered(String, String),
}

impl Event {
    /// (kind, a, b) for the QML `backendEvent(kind, a, b)` function.
    pub fn to_qml(&self) -> (&'static str, String, String) {
        match self {
            Event::Progress(id, p) => ("progress", id.clone(), format!("{p:.3}")),
            Event::Done(id, r) => ("done", id.clone(), r.clone()),
            Event::Installed(j) => ("installed", j.clone(), String::new()),
            Event::Removed(id, r) => ("removed", id.clone(), r.clone()),
            Event::CatalogAdd(j) => ("catalog-add", j.clone(), String::new()),
            Event::Launched(id, r) => ("launched", id.clone(), r.clone()),
            Event::Updates(j) => ("updates", j.clone(), String::new()),
            Event::History(j) => ("history", j.clone(), String::new()),
            Event::Details(id, j) => ("details", id.clone(), j.clone()),
            Event::Network(st, m) => ("network", st.clone(), m.clone()),
            Event::SearchDone(q, r) => ("search-done", q.clone(), r.clone()),
            Event::Notice(m) => ("notice", m.clone(), String::new()),
            Event::Registered(id, r) => ("registered", id.clone(), r.clone()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Flatpak { version: String },
    Simulated { reason: String },
}

impl Kind {
    pub fn mode(&self) -> &'static str {
        match self {
            Kind::Flatpak { .. } => "flatpak",
            Kind::Simulated { .. } => "simulated",
        }
    }

    pub fn status(&self) -> String {
        match self {
            Kind::Flatpak { version } => format!("{version} · Flathub, installs for this user"),
            Kind::Simulated { reason } => reason.clone(),
        }
    }
}

/// A `flatpak` command, routed through `flatpak-spawn --host` when running inside a sandbox.
fn flatpak(args: &[&str]) -> Command {
    let mut c = if std::env::var_os("FLATPAK_ID").is_some() {
        // flatpak-spawn does not pass our environment to the host: English output is asked for
        // explicitly, or the host answers in the user's language and parsing fails.
        let mut c = Command::new("flatpak-spawn");
        c.args(["--host", "--env=LC_ALL=C", "flatpak"]);
        c
    } else {
        Command::new("flatpak")
    };
    c.args(args).env("LC_ALL", "C").stdin(Stdio::null());
    c
}

/// Turns raw tool errors into a short, actionable message (PROJECT.md §5.3: actionable errors).
pub fn friendly_error(raw: &str) -> String {
    let l = raw.to_lowercase();
    let has = |words: &[&str]| words.iter().any(|w| l.contains(w));
    if has(&[
        "could not resolve",
        "couldn't resolve",
        "network is unreachable",
        "failed to connect",
        "connection timed out",
        "temporary failure in name resolution",
        "unable to connect",
        "no route to host",
    ]) {
        "No internet connection. Connect, then press A to retry".into()
    } else if has(&["no space left", "not enough free space", "not enough disk space"]) {
        let detail = raw
            .split_once("need")
            .map(|(_, d)| format!(" (need{d})"))
            .unwrap_or_default();
        format!("Not enough free space on the device{detail}")
    } else if has(&["checksum does not match"]) {
        "The download was damaged (checksum mismatch). Press A to retry".into()
    } else if has(&["rate limit", "error: 403", "error: 429", " 403", " 429"]) && has(&["github", "403", "429", "rate"])
    {
        "GitHub's hourly limit was reached. Try again later".into()
    } else {
        raw.trim().trim_start_matches("error: ").to_string()
    }
}

/// Flathub app IDs are reverse-DNS names; anything else is refused before reaching a shell.
pub fn valid_app_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 255
        && id.contains('.')
        && !id.starts_with('-')
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Asks the running Steam client to show its on-screen keyboard (Gaming Mode). Steam then types
/// into the focused window with ordinary key presses. Returns at once; the request is reaped on
/// its own thread. Not yet checked on a Steam Deck.
pub fn open_steam_keyboard() -> Result<(), String> {
    let mut child = host(&["steam", "steam://open/keyboard"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not ask Steam ({e})"))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn shell_quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', "'\\''"))
}

fn script_available() -> bool {
    Command::new("script")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// A program on the host system (through `flatpak-spawn --host` inside the sandbox).
fn host_argv<'a>(argv: &[&'a str]) -> Vec<&'a str> {
    let mut v: Vec<&str> = Vec::new();
    if std::env::var_os("FLATPAK_ID").is_some() {
        // The host does not inherit our environment (see `flatpak`); keep its output in English.
        v.extend(["flatpak-spawn", "--host", "--env=LC_ALL=C"]);
    }
    v.extend_from_slice(argv);
    v
}

fn host(argv: &[&str]) -> Command {
    let v = host_argv(argv);
    let mut c = Command::new(v[0]);
    c.args(&v[1..]).env("LC_ALL", "C").stdin(Stdio::null());
    c
}

/// `flatpak <args>` for a long-running job. Piped flatpak prints no progress, so when the
/// util-linux `script` tool exists the command runs on a pseudo-terminal, where flatpak shows
/// its normal "NN%" progress (stdout and stderr then arrive together on stdout).
fn flatpak_with_progress(args: &[&str]) -> Command {
    let mut argv = vec!["flatpak"];
    argv.extend_from_slice(args);
    with_progress(host_argv(&argv))
}

/// Runs `argv` on a pseudo-terminal when possible, so tools show their progress bars.
fn with_progress(mut argv: Vec<&str>) -> Command {
    let mut c = if script_available() {
        // `--noninteractive` also turns the progress display off, so it is dropped on a
        // pseudo-terminal; `-y` still answers every question and stdin stays closed.
        argv.retain(|a| *a != "--noninteractive");
        let line: Vec<String> = argv.iter().map(|a| shell_quote(a)).collect();
        let mut c = Command::new("script");
        c.args(["-q", "-e", "-f", "-c", &line.join(" "), "/dev/null"]);
        c
    } else {
        let mut c = Command::new(argv[0]);
        c.args(&argv[1..]);
        c
    };
    // `script -c` runs the line with $SHELL; the quoting above is POSIX sh, so pin it (the
    // user's login shell may be fish).
    c.env("LC_ALL", "C")
        .env("COLUMNS", "120")
        .env("SHELL", "/bin/sh")
        .stdin(Stdio::null());
    c
}

/// Removes terminal escape sequences (colours, cursor moves) from pseudo-terminal output.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for n in chars.by_ref() {
                    if ('@'..='~').contains(&n) {
                        break;
                    }
                }
            } else {
                chars.next();
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// `flatpak run <id>`, detached. Unlike the other commands it keeps the user's locale.
fn launch_flatpak(id: &str) -> Result<(), String> {
    if !valid_app_id(id) {
        return Err(format!("not a valid app ID: {id}"));
    }
    let mut c = if std::env::var_os("FLATPAK_ID").is_some() {
        let mut c = Command::new("flatpak-spawn");
        c.args(["--host", "flatpak", "run", id]);
        c
    } else {
        let mut c = Command::new("flatpak");
        c.args(["run", id]);
        c
    };
    let mut child = c
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("could not run flatpak: {e}"))?;
    // Reap it when it ends so it does not linger as a zombie.
    thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// Version, sizes and permissions of a Flathub app (read-only `flatpak remote-info`).
pub fn flathub_details(id: &str) -> crate::details::AppDetails {
    if !valid_app_id(id) {
        return crate::details::AppDetails {
            id: id.into(),
            error: "not a Flathub app ID".into(),
            ..Default::default()
        };
    }
    // SteamOS sets Flathub up system-wide; the user's own "flathub" remote is only added by the
    // first install, so before that the details are read from the system one (read-only).
    let (info, scope) = match run(&["remote-info", "--user", "flathub", id]) {
        Ok(i) => (Ok(i), "--user"),
        Err(_) => (run(&["remote-info", "--system", "flathub", id]), "--system"),
    };
    let meta = run(&["remote-info", scope, "--show-metadata", "flathub", id]);
    match (info, meta) {
        (Ok(i), Ok(m)) => crate::details::build(id, &i, &m),
        (Err(e), _) | (_, Err(e)) => {
            tracing::warn!(id, "details unavailable: {e}");
            crate::details::AppDetails {
                id: id.into(),
                error: e,
                ..Default::default()
            }
        }
    }
}

fn run(args: &[&str]) -> Result<String, String> {
    let out = flatpak(args)
        .output()
        .map_err(|e| format!("could not run flatpak: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(last_line(&err).unwrap_or("flatpak failed").to_string())
    }
}

fn last_line(s: &str) -> Option<&str> {
    s.lines().map(str::trim).rfind(|l| !l.is_empty())
}

pub fn detect(force_simulated: Option<&str>) -> Kind {
    if let Some(reason) = force_simulated {
        return Kind::Simulated {
            reason: reason.to_string(),
        };
    }
    match run(&["--version"]) {
        Ok(v) => Kind::Flatpak {
            version: v.trim().to_string(),
        },
        Err(_) => Kind::Simulated {
            reason: "Flatpak not found · showing simulated sample items".to_string(),
        },
    }
}

/// Parses tab-separated `flatpak list/search --columns=...` output (no header when piped).
pub fn parse_columns(out: &str, n: usize) -> Vec<Vec<String>> {
    out.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split('\t').map(|c| c.trim().to_string()).collect::<Vec<_>>())
        .filter(|cols| cols.len() >= n && !cols[0].is_empty())
        .collect()
}

/// Arxis Deck's own Flatpak app ID; it is left out of the Installed list.
pub const SELF_ID: &str = "io.github.fatih_bucaklioglu.ArxisDeck";

pub fn installed_json() -> Result<String, String> {
    let out = run(&["list", "--user", "--app", "--columns=application,name,version"])?;
    let items: Vec<String> = parse_columns(&out, 2)
        .iter()
        .filter(|c| c[0] != SELF_ID)
        .map(|c| {
            json::object(&[
                ("id", &c[0]),
                ("name", &c[1]),
                ("version", c.get(2).map(String::as_str).unwrap_or("")),
                ("icon", &icon_url(&c[0])),
                ("source", "flatpak"),
            ])
        })
        .chain(compat_tools().iter().map(|name| {
            json::object(&[
                ("id", &format!("{COMPAT_PREFIX}{name}")),
                ("name", name),
                ("version", "Steam compatibility tool"),
                ("icon", ""),
                ("source", "compat"),
            ])
        }))
        .collect();
    Ok(json::array(&items))
}

/// Raw Proton-GE releases JSON from GitHub (read-only; fetched by the host's curl).
pub fn ge_releases_json() -> Result<String, String> {
    let o = host(&[
        "curl",
        "-fsSL",
        "--max-time",
        "20",
        "-H",
        "Accept: application/vnd.github+json",
        GE_PROTON_RELEASES,
    ])
    .output()
    .map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

/// Folders in Steam's compatibility tools folder (Proton-GE and similar), sorted.
pub fn compat_tools() -> Vec<String> {
    let script = format!(
        "{COMPAT_DIR_SH}\n[ -d \"$d\" ] || exit 0\nfor x in \"$d\"/*/; do [ -d \"$x\" ] && basename \"$x\"; done\n"
    );
    let Ok(out) = host(&["sh", "-c", &script]).output() else {
        return Vec::new();
    };
    let mut names: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|n| valid_tool_name(n))
        .map(String::from)
        .collect();
    names.sort();
    names
}

pub fn search_json(query: &str) -> Result<String, String> {
    let out = run(&["search", "--columns=application,name,description,remotes", query])?;
    let items: Vec<String> = parse_columns(&out, 4)
        .iter()
        .filter(|c| c[3].split(',').any(|r| r.trim() == "flathub"))
        .take(40)
        .map(|c| {
            json::object(&[
                ("id", &c[0]),
                ("name", &c[1]),
                ("category", "Flathub"),
                ("summary", &c[2]),
                ("icon", &icon_url(&c[0])),
            ])
        })
        .collect();
    Ok(json::array(&items))
}

/// Tracks overall progress from flatpak's install output: "Installing 2/3…" steps and "NN%".
#[derive(Default)]
pub struct ProgressParser {
    step: u32,
    steps: u32,
    pct: f32,
}

impl ProgressParser {
    /// Feeds a chunk of output; returns the overall progress 0..1 if it could be read.
    pub fn feed(&mut self, chunk: &str) -> Option<f32> {
        let mut seen = false;
        for seg in chunk.split(['\r', '\n']) {
            for word in seg.split_whitespace() {
                // In the C locale flatpak prints its "…" as "?".
                let w = word.trim_end_matches(['…', '.', ':', '?']);
                if let Some((a, b)) = w.split_once('/') {
                    if let (Ok(a), Ok(b)) = (a.parse::<u32>(), b.parse::<u32>()) {
                        if b > 0 && a >= 1 && a <= b && (seg.contains("Installing") || seg.contains("Updating")) {
                            if a != self.step {
                                self.pct = 0.0;
                            }
                            self.step = a;
                            self.steps = b;
                            seen = true;
                        }
                    }
                }
                // flatpak's own estimate wobbles (85 % → 66 % → 84 %); within a step the
                // bar only moves forward.
                if let Some(p) = w.strip_suffix('%').and_then(|n| n.parse::<f32>().ok()) {
                    if (0.0..=100.0).contains(&p) {
                        self.pct = self.pct.max(p / 100.0);
                        seen = true;
                    }
                }
            }
        }
        if !seen {
            return None;
        }
        let overall = if self.steps > 0 {
            ((self.step - 1) as f32 + self.pct) / self.steps as f32
        } else {
            self.pct
        };
        Some(overall.clamp(0.0, 0.99))
    }
}

struct Shared {
    kind: Kind,
    events: Sender<Event>,
    queue: Mutex<VecDeque<(String, String, Job)>>,
    wake: Condvar,
    cancelled: Mutex<HashSet<String>>,
    current: Mutex<Option<(String, Option<Child>)>>,
    sim_installed: Mutex<Vec<(String, String)>>,
    sim_updated: Mutex<HashSet<String>>,
    sim_offline: Mutex<bool>,
    online: Mutex<Option<bool>>,
    recheck: std::sync::atomic::AtomicBool,
    appimages: Mutex<crate::appimage::Manifest>,
    manifest_path: Option<std::path::PathBuf>,
    flathub_ready: Mutex<bool>,
    search_gen: AtomicU64,
    history: Mutex<Option<History>>,
}

impl Shared {
    fn send(&self, e: Event) {
        let _ = self.events.send(e);
    }

    /// Where an operation on `id` comes from, for the history.
    fn source_of(&self, id: &str) -> &'static str {
        match (&self.kind, id.starts_with(COMPAT_PREFIX)) {
            (Kind::Simulated { .. }, _) => "simulated",
            (_, true) => "github",
            (_, false) => "flatpak",
        }
    }

    /// What the install confirmation shows (crate::details).
    fn details(&self, id: &str) -> crate::details::AppDetails {
        if let Kind::Simulated { .. } = self.kind {
            return crate::details::simulated(id);
        }
        flathub_details(id)
    }

    fn hist_start(&self, kind: &str, id: &str, name: &str) -> Option<i64> {
        let op = {
            let h = self.history.lock().unwrap();
            h.as_ref()?.start(kind, id, name, self.source_of(id))
        };
        self.send_history(); // shows it as running
        op
    }

    fn hist_finish(&self, op: Option<i64>, result: &str) {
        if let Some(h) = self.history.lock().unwrap().as_ref() {
            h.finish(op, result);
        }
        self.send_history();
    }

    /// The 50 newest operations, newest first, to the UI.
    fn send_history(&self) {
        if let Some(h) = self.history.lock().unwrap().as_ref() {
            self.send(Event::History(h.recent_json(50)));
        }
    }

    /// Simulated store: sample items with an even number have an update until updated.
    fn sim_has_update(&self, id: &str) -> bool {
        let n: u32 = id
            .trim_start_matches(|c: char| !c.is_ascii_digit())
            .parse()
            .unwrap_or(1);
        n.is_multiple_of(2) && !self.sim_updated.lock().unwrap().contains(id)
    }

    fn check_updates(&self) {
        match &self.kind {
            Kind::Flatpak { .. } => match run(&[
                "remote-ls",
                "--user",
                "--updates",
                "--app",
                "--columns=application,version",
            ]) {
                Ok(out) => {
                    let items: Vec<String> = parse_columns(&out, 1)
                        .iter()
                        .filter(|c| c[0] != SELF_ID)
                        .map(|c| {
                            json::object(&[("id", &c[0]), ("version", c.get(1).map(String::as_str).unwrap_or(""))])
                        })
                        .collect();
                    self.send(Event::Updates(json::array(&items)));
                }
                Err(e) => tracing::warn!("checking for updates failed: {e}"),
            },
            Kind::Simulated { .. } => {
                let ids: Vec<String> = self
                    .sim_installed
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect();
                let items: Vec<String> = ids
                    .iter()
                    .filter(|id| self.sim_has_update(id))
                    .map(|id| json::object(&[("id", id), ("version", "simulated")]))
                    .collect();
                self.send(Event::Updates(json::array(&items)));
            }
        }
    }

    /// Proton-GE releases → catalogue cards (asset rules in `github::choose`).
    fn fetch_releases(&self) {
        let raw = match &self.kind {
            Kind::Flatpak { .. } => ge_releases_json(),
            Kind::Simulated { .. } => Ok(sim_releases_json()),
        };
        match raw.and_then(|j| crate::github::proton_ge_items(&j, 3)) {
            Ok(items) => {
                tracing::info!(count = items.len(), "Proton-GE releases offered");
                match serde_json::to_string(&items) {
                    Ok(j) => self.send(Event::CatalogAdd(j)),
                    Err(e) => tracing::warn!("Proton-GE releases: {e}"),
                }
            }
            Err(e) => {
                tracing::warn!("Proton-GE releases: {e}");
                self.send(Event::Notice(format!(
                    "Proton-GE list unavailable: {}",
                    friendly_error(&e)
                )));
            }
        }
        self.fetch_appimages();
    }

    /// Curated AppImages: the latest release of each project → catalogue cards.
    fn fetch_appimages(&self) {
        let items: Vec<crate::appimage::Item> = match &self.kind {
            Kind::Simulated { .. } => crate::appimage::simulated_items(),
            Kind::Flatpak { .. } => {
                let mut out = Vec::new();
                for (repo, name, category, summary) in crate::appimage::CURATED {
                    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
                    let r = host(&[
                        "curl",
                        "-fsSL",
                        "--max-time",
                        "20",
                        "-H",
                        "Accept: application/vnd.github+json",
                        &url,
                    ])
                    .output()
                    .map_err(|e| e.to_string())
                    .and_then(|o| {
                        if o.status.success() {
                            Ok(String::from_utf8_lossy(&o.stdout).into_owned())
                        } else {
                            Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
                        }
                    })
                    .and_then(|j| crate::appimage::item_from_release(&j, repo, name, category, summary));
                    match r {
                        Ok(it) => out.push(it),
                        Err(e) => tracing::warn!(repo, "AppImage release: {e}"),
                    }
                }
                out
            }
        };
        tracing::info!(count = items.len(), "AppImages offered");
        if let Ok(j) = serde_json::to_string(&items) {
            self.send(Event::CatalogAdd(j));
        }
    }

    fn refresh_installed(&self) {
        match &self.kind {
            Kind::Flatpak { .. } => match installed_json() {
                Ok(j) => self.send(Event::Installed(self.with_appimages(&j))),
                Err(e) => tracing::warn!("flatpak list failed: {e}"),
            },
            Kind::Simulated { .. } => {
                let items: Vec<String> = self
                    .sim_installed
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(id, name)| {
                        let source = if id.starts_with(COMPAT_PREFIX) {
                            "compat"
                        } else {
                            "flatpak"
                        };
                        json::object(&[("id", id), ("name", name), ("version", "simulated"), ("source", source)])
                    })
                    .collect();
                self.send(Event::Installed(self.with_appimages(&json::array(&items))));
            }
        }
    }

    /// Adds the AppImages from the manifest to an installed-list JSON array.
    fn with_appimages(&self, base: &str) -> String {
        let mut list: Vec<serde_json::Value> = serde_json::from_str(base).unwrap_or_default();
        let entries = self.appimages.lock().unwrap().entries.clone();
        let home = std::env::var("HOME").unwrap_or_default();
        for e in entries {
            let missing = matches!(self.kind, Kind::Flatpak { .. })
                && !host(&["test", "-f", &e.path]).status().is_ok_and(|st| st.success());
            list.push(serde_json::json!({
                "id": e.id,
                "name": e.name,
                "version": e.version,
                "source": "appimage",
                "icon": "",
                "registered": if e.registered { "true" } else { "false" },
                "missing": if missing { "true" } else { "false" },
                "path": crate::logging::redact(&e.path, &home),
            }));
        }
        serde_json::Value::Array(list).to_string()
    }

    fn manifest_add(&self, id: &str, name: &str, version: &str, repo: &str, file: &str, sha256: &str) {
        let home = std::env::var("HOME").unwrap_or_default();
        let mut m = self.appimages.lock().unwrap();
        m.entries.retain(|e| e.id != id);
        m.entries.push(crate::appimage::Entry {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            repo: repo.into(),
            path: format!("{home}/Applications/{file}"),
            sha256: sha256.into(),
            registered: false,
            desktop: None,
        });
        if let Err(e) = m.save(self.manifest_path.as_deref()) {
            tracing::warn!("AppImage manifest: {e}");
        }
    }

    fn install_appimage(&self, id: &str, url: &str, sha256: &str, file: &str, size: u64, need_kb: u64) -> String {
        if !crate::appimage::valid_file_name(file) || !valid_release_url(url) {
            return "not a valid AppImage download".into();
        }
        if !sha256.is_empty() && !(sha256.len() == 64 && sha256.chars().all(|c| c.is_ascii_hexdigit())) {
            return "invalid checksum".into();
        }
        let (need, size) = (need_kb.to_string(), size.to_string());
        let argv = host_argv(&[
            "sh",
            "-c",
            crate::appimage::INSTALL_SH,
            "sh",
            url,
            sha256,
            file,
            &need,
            &size,
        ]);
        self.run_job(id, with_progress(argv))
    }

    /// "Make runnable": chmod 755 + start-menu entry, only for files in the manifest.
    fn register_appimage(&self, id: &str) -> String {
        let Some(e) = self.appimages.lock().unwrap().get(id).cloned() else {
            return "not an AppImage installed by Arxis Deck".into();
        };
        let desktop_file = crate::appimage::desktop_name(id);
        let desktop = match &self.kind {
            Kind::Simulated { .. } => format!("simulated/{desktop_file}"),
            Kind::Flatpak { .. } => {
                match host(&[
                    "sh",
                    "-c",
                    crate::appimage::REGISTER_SH,
                    "sh",
                    &e.path,
                    &desktop_file,
                    &e.name,
                ])
                .output()
                {
                    Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                        .lines()
                        .find_map(|l| l.strip_prefix("desktop:"))
                        .unwrap_or_default()
                        .to_string(),
                    Ok(o) => return friendly_error(String::from_utf8_lossy(&o.stdout).trim()),
                    Err(e) => return e.to_string(),
                }
            }
        };
        let mut m = self.appimages.lock().unwrap();
        if let Some(entry) = m.entries.iter_mut().find(|x| x.id == id) {
            entry.registered = true;
            entry.desktop = Some(desktop);
        }
        match m.save(self.manifest_path.as_deref()) {
            Ok(()) => "ok".into(),
            Err(e) => e,
        }
    }

    /// Uninstall: only the files the manifest lists (the AppImage and our .desktop file).
    fn uninstall_appimage(&self, id: &str) -> String {
        let Some(e) = self.appimages.lock().unwrap().get(id).cloned() else {
            return "not an AppImage installed by Arxis Deck".into();
        };
        if let Kind::Flatpak { .. } = self.kind {
            let mut files = vec![e.path.clone()];
            files.extend(e.desktop.clone());
            let mut argv = vec!["sh", "-c", crate::appimage::REMOVE_SH, "sh"];
            argv.extend(files.iter().map(String::as_str));
            match host(&argv).output() {
                Ok(o) if o.status.success() => {}
                Ok(o) => {
                    return String::from_utf8_lossy(&o.stdout)
                        .trim()
                        .trim_start_matches("error: ")
                        .to_string()
                }
                Err(err) => return err.to_string(),
            }
        }
        let mut m = self.appimages.lock().unwrap();
        m.entries.retain(|x| x.id != id);
        match m.save(self.manifest_path.as_deref()) {
            Ok(()) => "ok".into(),
            Err(err) => err,
        }
    }

    fn launch_appimage(&self, id: &str) -> String {
        let Some(e) = self.appimages.lock().unwrap().get(id).cloned() else {
            return "not an AppImage installed by Arxis Deck".into();
        };
        if !e.registered {
            return "make it runnable first (A in Installed)".into();
        }
        if matches!(self.kind, Kind::Simulated { .. }) {
            return "ok".into();
        }
        let script = r#"command -v fusermount >/dev/null 2>&1 || command -v fusermount3 >/dev/null 2>&1 || export APPIMAGE_EXTRACT_AND_RUN=1
exec "$1""#;
        // Keeps the user's language: not routed through `host()`, which forces English output.
        let mut argv: Vec<&str> = Vec::new();
        if std::env::var_os("FLATPAK_ID").is_some() {
            argv.extend(["flatpak-spawn", "--host"]);
        }
        argv.extend(["sh", "-c", script, "sh", &e.path]);
        match Command::new(argv[0])
            .args(&argv[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(mut child) => {
                thread::spawn(move || {
                    let _ = child.wait();
                });
                "ok".into()
            }
            Err(e) => e.to_string(),
        }
    }

    fn take_cancel(&self, id: &str) -> bool {
        self.cancelled.lock().unwrap().remove(id)
    }

    /// Whether Flathub can be reached (simulated store: the test switch).
    fn reachable(&self) -> bool {
        match &self.kind {
            Kind::Simulated { .. } => !*self.sim_offline.lock().unwrap(),
            Kind::Flatpak { .. } => host(&["curl", "-fsSI", "--max-time", "6", "-o", "/dev/null", FLATHUB_REPO])
                .status()
                .is_ok_and(|st| st.success()),
        }
    }

    /// Checks the connection and tells the UI when it changes; after coming back online the
    /// lists that need the network are refreshed.
    fn check_network(&self) -> bool {
        let up = self.reachable();
        let before = self.online.lock().unwrap().replace(up);
        if before != Some(up) {
            tracing::info!(online = up, "network state");
            let msg = if up {
                "Flathub can be reached"
            } else {
                "Can't reach Flathub. Installed apps still work; downloads wait until you're back online."
            };
            self.send(Event::Network(if up { "online" } else { "offline" }.into(), msg.into()));
            if up && before == Some(false) {
                self.fetch_releases();
                self.check_updates();
            }
        }
        up
    }

    fn install_sim(&self, id: &str, name: &str) -> String {
        if *self.sim_offline.lock().unwrap() {
            return "Could not resolve host: dl.flathub.org".into();
        }
        let mut p = 0.0f32;
        loop {
            thread::sleep(Duration::from_millis(SIM_STEP_MS));
            if self.take_cancel(id) {
                return "cancelled".into();
            }
            p = (p + SIM_STEP).min(1.0);
            if p >= 1.0 {
                let mut list = self.sim_installed.lock().unwrap();
                if !list.iter().any(|(i, _)| i == id) {
                    list.push((id.to_string(), name.to_string()));
                }
                return "ok".into();
            }
            self.send(Event::Progress(id.to_string(), p));
        }
    }

    fn ensure_flathub(&self) -> Result<(), String> {
        let mut ready = self.flathub_ready.lock().unwrap();
        if !*ready {
            run(&["remote-add", "--user", "--if-not-exists", "flathub", FLATHUB_REPO])?;
            *ready = true;
        }
        Ok(())
    }

    fn install_flatpak(&self, id: &str, update: bool) -> String {
        if !valid_app_id(id) {
            return format!("not a valid app ID: {id}");
        }
        if let Err(e) = self.ensure_flathub() {
            return format!("could not set up Flathub: {e}");
        }
        let args: &[&str] = if update {
            &["update", "--user", "--noninteractive", "-y", id]
        } else {
            &["install", "--user", "--noninteractive", "-y", "flathub", id]
        };
        self.run_job(id, flatpak_with_progress(args))
    }

    fn install_compat(&self, id: &str, url: &str, sum: &str, need_kb: u64) -> String {
        let Some(name) = id.strip_prefix(COMPAT_PREFIX).filter(|n| valid_tool_name(n)) else {
            return format!("not a valid tool name: {id}");
        };
        if !valid_release_url(url) || !(sum.is_empty() || valid_release_url(sum)) {
            return "download link is not a GitHub release".into();
        }
        let script = format!("{COMPAT_DIR_SH}\n{COMPAT_INSTALL_SH}");
        let need = need_kb.to_string();
        let argv = host_argv(&["sh", "-c", &script, "sh", url, sum, name, &need]);
        self.run_job(id, with_progress(argv))
    }

    /// Runs one install command, streaming its progress; returns "ok", "cancelled" or an error.
    fn run_job(&self, id: &str, mut command: Command) -> String {
        let trace = std::env::var_os("ARXIS_FLATPAK_TRACE").is_some();
        let mut child = match command.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            Ok(c) => c,
            Err(e) => return format!("could not start the install: {e}"),
        };
        let mut stdout = child.stdout.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let err_text = thread::spawn(move || {
            let mut s = String::new();
            let _ = stderr.read_to_string(&mut s);
            s
        });
        *self.current.lock().unwrap() = Some((id.to_string(), Some(child)));
        let cancelled_early = self.take_cancel(id);
        if cancelled_early {
            self.kill_current(id);
        }
        let mut parser = ProgressParser::default();
        let mut buf = [0u8; 4096];
        let mut last = -1.0f32;
        let mut tail = String::new(); // recent output, for the error message on a pseudo-terminal
        while let Ok(n) = stdout.read(&mut buf) {
            if n == 0 {
                break;
            }
            let text = strip_ansi(&String::from_utf8_lossy(&buf[..n]));
            if trace {
                tracing::info!(output = ?text, "job output");
            }
            tail.push_str(&text);
            if tail.len() > 4000 {
                let cut = tail.len() - 2000;
                let cut = (cut..tail.len()).find(|i| tail.is_char_boundary(*i)).unwrap_or(0);
                tail.drain(..cut);
            }
            if let Some(p) = parser.feed(&text) {
                if (p - last).abs() >= 0.01 {
                    last = p;
                    self.send(Event::Progress(id.to_string(), p));
                }
            }
        }
        let child = self.current.lock().unwrap().take().and_then(|(_, c)| c);
        let status = child.map(|mut c| c.wait());
        let errors = err_text.join().unwrap_or_default();
        if cancelled_early || self.take_cancel(id) {
            return "cancelled".into();
        }
        match status {
            Some(Ok(s)) if s.success() => "ok".into(),
            Some(Ok(_)) => {
                let all = format!("{tail}\n{errors}");
                let line = all
                    .split(['\r', '\n'])
                    .map(str::trim)
                    .rfind(|l| l.to_lowercase().starts_with("error") || l.contains("rror:"))
                    .or_else(|| last_line(&errors))
                    .unwrap_or("install failed");
                line.to_string()
            }
            _ => "cancelled".into(),
        }
    }

    fn kill_current(&self, id: &str) {
        if let Some((cur, Some(child))) = self.current.lock().unwrap().as_mut() {
            if cur == id {
                let _ = child.kill();
            }
        }
    }

    fn uninstall(&self, id: &str) -> String {
        if id.starts_with(crate::appimage::PREFIX) {
            return self.uninstall_appimage(id);
        }
        if let Some(name) = id.strip_prefix(COMPAT_PREFIX) {
            if !valid_tool_name(name) {
                return format!("not a valid tool name: {name}");
            }
            return match &self.kind {
                Kind::Flatpak { .. } => {
                    let script = format!("{COMPAT_DIR_SH}\n{COMPAT_REMOVE_SH}");
                    match host(&["sh", "-c", &script, "sh", name]).output() {
                        Ok(o) if o.status.success() => "ok".into(),
                        Ok(o) => last_line(&String::from_utf8_lossy(&o.stderr))
                            .unwrap_or("could not remove it")
                            .to_string(),
                        Err(e) => e.to_string(),
                    }
                }
                Kind::Simulated { .. } => {
                    self.sim_installed.lock().unwrap().retain(|(i, _)| i != id);
                    "ok".into()
                }
            };
        }
        if !valid_app_id(id) {
            return format!("not a valid app ID: {id}");
        }
        match &self.kind {
            Kind::Flatpak { .. } => match run(&["uninstall", "--user", "--noninteractive", "-y", id]) {
                Ok(_) => "ok".into(),
                Err(e) => e,
            },
            Kind::Simulated { .. } => {
                self.sim_installed.lock().unwrap().retain(|(i, _)| i != id);
                "ok".into()
            }
        }
    }
}

/// Starts the backend threads. Send commands on the returned channel; events arrive on `events`.
/// `history`: where operations are recorded (None = not recorded).
pub fn start(kind: Kind, history: Option<History>) -> (Sender<Cmd>, Receiver<Event>) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
    let (ev_tx, ev_rx) = mpsc::channel::<Event>();
    // The AppImage manifest lives in the user's data folder; the simulated store keeps it in memory.
    let manifest_path = match kind {
        Kind::Flatpak { .. } => crate::appimage::manifest_path(),
        Kind::Simulated { .. } => None,
    };
    let shared = Arc::new(Shared {
        history: Mutex::new(history),
        kind,
        events: ev_tx,
        queue: Mutex::new(VecDeque::new()),
        wake: Condvar::new(),
        cancelled: Mutex::new(HashSet::new()),
        current: Mutex::new(None),
        sim_installed: Mutex::new(Vec::new()),
        sim_updated: Mutex::new(HashSet::new()),
        sim_offline: Mutex::new(false),
        online: Mutex::new(None),
        recheck: std::sync::atomic::AtomicBool::new(false),
        appimages: Mutex::new(crate::appimage::Manifest::load(manifest_path.as_deref())),
        manifest_path,
        flathub_ready: Mutex::new(false),
        search_gen: AtomicU64::new(0),
    });

    // Installs run one at a time, in order.
    let s = shared.clone();
    thread::spawn(move || loop {
        let (id, name, job) = {
            let mut q = s.queue.lock().unwrap();
            while q.is_empty() {
                q = s.wake.wait(q).unwrap();
            }
            q.pop_front().unwrap()
        };
        if s.take_cancel(&id) {
            s.send(Event::Done(id, "cancelled".into()));
            continue;
        }
        let update = job == Job::Flatpak { update: true };
        tracing::info!(id = %id, job = ?job, "install job started");
        let kind_name = match &job {
            Job::Flatpak { update: true } => "update",
            Job::Flatpak { update: false } => "install",
            Job::Compat { .. } | Job::AppImage { .. } => "install",
        };
        let op = s.hist_start(kind_name, &id, &name);
        let result = match (&s.kind, &job) {
            (Kind::Flatpak { .. }, Job::Flatpak { update }) => s.install_flatpak(&id, *update),
            (Kind::Flatpak { .. }, Job::Compat { url, sum, need_kb }) => s.install_compat(&id, url, sum, *need_kb),
            (
                Kind::Flatpak { .. },
                Job::AppImage {
                    url,
                    sha256,
                    file,
                    size,
                    need_kb,
                    ..
                },
            ) => s.install_appimage(&id, url, sha256, file, *size, *need_kb),
            (Kind::Simulated { .. }, _) => {
                let r = s.install_sim(&id, &name);
                if r == "ok" && update {
                    s.sim_updated.lock().unwrap().insert(id.clone());
                }
                if matches!(job, Job::AppImage { .. }) {
                    // AppImages are listed from the manifest, not as simulated Flatpaks.
                    s.sim_installed.lock().unwrap().retain(|(i, _)| *i != id);
                }
                r
            }
        };
        if result == "ok" {
            if let Job::AppImage {
                sha256,
                file,
                version,
                repo,
                ..
            } = &job
            {
                s.manifest_add(&id, &name, version, repo, file, sha256);
            }
        }
        let result = if result == "ok" || result == "cancelled" {
            result
        } else {
            friendly_error(&result)
        };
        let ok = result == "ok";
        tracing::info!(id = %id, result = %result, "install job finished");
        s.hist_finish(op, &result);
        s.send(Event::Done(id, result));
        if ok {
            s.refresh_installed();
            s.check_updates();
        }
    });

    // Commands.
    let s = shared.clone();
    thread::spawn(move || {
        s.send_history();
        s.refresh_installed();
        let s2 = s.clone();
        thread::spawn(move || s2.check_updates());
        let s3 = s.clone();
        thread::spawn(move || s3.fetch_releases());
        // Connection monitor: every 30 s while offline, every 5 minutes while online, or at once
        // when asked (CheckNetwork, the test switch).
        let s4 = s.clone();
        thread::spawn(move || loop {
            let up = s4.check_network();
            let wait = if up { 300 } else { 30 };
            for _ in 0..wait * 4 {
                thread::sleep(Duration::from_millis(250));
                if s4.recheck.swap(false, Ordering::SeqCst) {
                    break;
                }
            }
        });
        for cmd in cmd_rx {
            match cmd {
                Cmd::Install { id, name, job } => {
                    s.cancelled.lock().unwrap().remove(&id);
                    s.queue.lock().unwrap().push_back((id, name, job));
                    s.wake.notify_one();
                }
                Cmd::Cancel(id) => {
                    let mut q = s.queue.lock().unwrap();
                    if let Some(pos) = q.iter().position(|(i, _, _)| *i == id) {
                        q.remove(pos);
                        drop(q);
                        s.send(Event::Done(id, "cancelled".into()));
                    } else {
                        drop(q);
                        s.cancelled.lock().unwrap().insert(id.clone());
                        s.kill_current(&id);
                    }
                }
                Cmd::Uninstall(id) => {
                    let s2 = s.clone();
                    thread::spawn(move || {
                        let op = s2.hist_start("uninstall", &id, &id);
                        let r = s2.uninstall(&id);
                        tracing::info!(id = %id, result = %r, "uninstall finished");
                        s2.hist_finish(op, &r);
                        s2.send(Event::Removed(id, r));
                        s2.refresh_installed();
                        s2.check_updates();
                    });
                }
                Cmd::Register(id) => {
                    let s2 = s.clone();
                    thread::spawn(move || {
                        let r = s2.register_appimage(&id);
                        tracing::info!(id = %id, result = %r, "AppImage made runnable");
                        let op = s2.hist_start("enable", &id, &id);
                        s2.hist_finish(op, &r);
                        s2.send(Event::Registered(id, r));
                        s2.refresh_installed();
                    });
                }
                Cmd::Launch(id) => {
                    let r = if id.starts_with(crate::appimage::PREFIX) {
                        s.launch_appimage(&id)
                    } else {
                        match &s.kind {
                            Kind::Flatpak { .. } => launch_flatpak(&id).map(|_| "ok".to_string()).unwrap_or_else(|e| e),
                            Kind::Simulated { .. } => "ok".to_string(),
                        }
                    };
                    tracing::info!(id = %id, result = %r, "launch");
                    let op = s.hist_start("launch", &id, &id);
                    s.hist_finish(op, &r);
                    s.send(Event::Launched(id, r));
                }
                Cmd::Details(id) => {
                    let s2 = s.clone();
                    thread::spawn(move || {
                        let d = s2.details(&id);
                        let j = serde_json::to_string(&d).unwrap_or_else(|_| "{}".into());
                        s2.send(Event::Details(id, j));
                    });
                }
                Cmd::CheckUpdates => {
                    let s2 = s.clone();
                    thread::spawn(move || s2.check_updates());
                }
                Cmd::CheckNetwork => s.recheck.store(true, Ordering::SeqCst),
                Cmd::SimOffline(off) => {
                    if matches!(s.kind, Kind::Simulated { .. }) {
                        *s.sim_offline.lock().unwrap() = off;
                        s.recheck.store(true, Ordering::SeqCst);
                    }
                }
                Cmd::Search(q) => {
                    if matches!(s.kind, Kind::Simulated { .. }) {
                        continue;
                    }
                    let gen = s.search_gen.fetch_add(1, Ordering::SeqCst) + 1;
                    let s2 = s.clone();
                    thread::spawn(move || {
                        let r = search_json(&q);
                        // Drop results of searches overtaken by a newer one.
                        if s2.search_gen.load(Ordering::SeqCst) != gen {
                            return;
                        }
                        match r {
                            Ok(j) => {
                                let n = serde_json::from_str::<serde_json::Value>(&j)
                                    .ok()
                                    .and_then(|v| v.as_array().map(|a| a.len()))
                                    .unwrap_or(0);
                                s2.send(Event::CatalogAdd(j));
                                s2.send(Event::SearchDone(q, format!("ok:{n}")));
                            }
                            Err(e) => {
                                tracing::warn!("flatpak search failed: {e}");
                                s2.send(Event::SearchDone(q, friendly_error(&e)));
                            }
                        }
                    });
                }
            }
        }
    });

    (cmd_tx, ev_rx)
}

/// Parses the QML `commands` text: one "verb<TAB>arg[<TAB>arg2]" per line.
pub fn parse_commands(text: &str) -> Vec<Cmd> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut parts = line.split('\t');
        let (verb, arg) = (
            parts.next().unwrap_or(""),
            parts.next().unwrap_or("").trim().to_string(),
        );
        let extra = parts.next().unwrap_or("").to_string();
        let (url, sum) = (
            parts.next().unwrap_or("").trim().to_string(),
            parts.next().unwrap_or("").trim().to_string(),
        );
        let need_kb: u64 = parts.next().unwrap_or("").trim().parse().unwrap_or(0);
        // install-appimage only: file, size, version, repository.
        let file = parts.next().unwrap_or("").trim().to_string();
        let size: u64 = parts.next().unwrap_or("").trim().parse().unwrap_or(0);
        let version = parts.next().unwrap_or("").trim().to_string();
        let repo = parts.next().unwrap_or("").trim().to_string();
        match verb {
            "check-updates" => {
                out.push(Cmd::CheckUpdates);
                continue;
            }
            "check-network" => {
                out.push(Cmd::CheckNetwork);
                continue;
            }
            _ => {}
        }
        if arg.is_empty() {
            continue;
        }
        out.push(match verb {
            "install" | "update" => Cmd::Install {
                name: if extra.is_empty() { arg.clone() } else { extra },
                job: Job::Flatpak {
                    update: verb == "update",
                },
                id: arg,
            },
            "install-appimage" => Cmd::Install {
                name: if extra.is_empty() { arg.clone() } else { extra },
                job: Job::AppImage {
                    url,
                    sha256: sum,
                    file,
                    size,
                    version,
                    repo,
                    need_kb,
                },
                id: arg,
            },
            "register" => Cmd::Register(arg),
            "install-compat" => Cmd::Install {
                name: if extra.is_empty() { arg.clone() } else { extra },
                job: Job::Compat { url, sum, need_kb },
                id: arg,
            },
            "check-updates" => Cmd::CheckUpdates,
            "cancel" => Cmd::Cancel(arg),
            "uninstall" => Cmd::Uninstall(arg),
            "search" => Cmd::Search(arg),
            "launch" => Cmd::Launch(arg),
            "details" => Cmd::Details(arg),
            "sim-offline" => Cmd::SimOffline(arg == "1"),
            _ => continue,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_are_tab_separated() {
        let rows = parse_columns("com.a.B\tB app\t1.0\n\norg.c.D\tD\t\n", 2);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], vec!["com.a.B", "B app", "1.0"]);
    }

    #[test]
    fn progress_from_steps_and_percent() {
        let mut p = ProgressParser::default();
        assert_eq!(p.feed("Looking for matches…\n"), None);
        assert_eq!(p.feed("Installing 1/2… "), Some(0.0));
        let half = p.feed("\rInstalling 1/2… ████      50%  1.2 MB/s").unwrap();
        assert!((half - 0.25).abs() < 1e-4);
        let second = p.feed("\nInstalling 2/2…\n").unwrap();
        assert!((second - 0.5).abs() < 1e-4);
        assert!(p.feed("Installing 2/2… 100%").unwrap() <= 0.99);
    }

    /// Lines as recorded from a real `flatpak install` on a pseudo-terminal (LC_ALL=C).
    #[test]
    fn real_flatpak_output_gives_steady_progress() {
        let lines = [
            "\rInstalling 1/2?                        0%  0 bytes/s",
            "\rInstalling 1/2? ?????????????         63%  8.4?kB/s",
            "\rInstalling 1/2? ?????????????????     85%  4.7?kB/s",
            "\rInstalling 1/2? ??????????????        66%  5.6?kB/s",
            "\rInstalling 1/2? ???????????????????? 100%  82.9?kB/s  00:00",
            "\rInstalling 2/2?                        0%  0 bytes/s",
            "\rInstalling 2/2? ?????????             43%  409.7?kB/s",
            "\rInstalling 2/2? ???????????????????? 100%  535.6?kB/s",
        ];
        let mut p = ProgressParser::default();
        let got: Vec<f32> = lines.iter().filter_map(|l| p.feed(l)).collect();
        assert_eq!(got.len(), lines.len());
        assert!(got.windows(2).all(|w| w[1] >= w[0]), "progress went backwards: {got:?}");
        assert!((got[1] - 0.315).abs() < 1e-3); // 63 % of step 1 of 2
        assert!((got[5] - 0.5).abs() < 1e-3); // step 2 starts at half way
        assert!(got[7] <= 0.99);
    }

    #[test]
    fn terminal_output_is_cleaned_and_parsed() {
        let raw = "\u{1b}[1mInstalling 1/1…\u{1b}[0m \u{1b}[32m████\u{1b}[0m  42%  3.1 MB/s\u{1b}[K\r";
        let clean = strip_ansi(raw);
        assert!(!clean.contains('\u{1b}'));
        let p = ProgressParser::default().feed(&clean).unwrap();
        assert!((p - 0.42).abs() < 1e-4);
    }

    #[test]
    fn app_ids_are_checked_and_quoted() {
        assert!(valid_app_id("org.gnome.TextEditor"));
        assert!(valid_app_id("com.steamgriddb.steam-rom-manager"));
        assert!(!valid_app_id("org.x; rm -rf ~"));
        assert!(!valid_app_id("--help"));
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn compat_tool_names_and_links_are_checked() {
        assert!(valid_tool_name("GE-Proton11-12"));
        assert!(!valid_tool_name("../steamapps"));
        assert!(!valid_tool_name("a/b"));
        assert!(!valid_tool_name(".hidden"));
        let url = "https://github.com/GloriousEggroll/proton-ge-custom/releases/download/GE-Proton11-12/GE-Proton11-12-x86_64.tar.gz";
        assert!(valid_release_url(url));
        assert!(!valid_release_url("https://example.com/releases/download/x.tar.gz"));
        assert!(!valid_release_url(
            "https://github.com/a/b/releases/download/x;$(id).tar.gz"
        ));
    }

    /// Runs the real install script against fixture archives in a temporary HOME.
    #[test]
    fn compat_install_script_is_safe() {
        use std::process::Command;
        let root = std::env::temp_dir().join(format!("arxis-compat-test-{}", std::process::id()));
        let src = root.join("src");
        let home = root.join("home");
        std::fs::create_dir_all(src.join("Tool-1")).unwrap();
        std::fs::create_dir_all(src.join("Other")).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(src.join("Tool-1/proton"), "x").unwrap();
        std::fs::write(src.join("Other/file"), "x").unwrap();
        let tar = |name: &str, extra: &[&str]| {
            let out = root.join(name);
            let mut c = Command::new("tar");
            c.arg("-czf").arg(&out).arg("-C").arg(&src).args(extra);
            assert!(c.status().unwrap().success());
            format!("file://{}", out.display())
        };
        let good = tar("good.tar.gz", &["Tool-1"]);
        let outside = tar("outside.tar.gz", &["Tool-1", "Other"]);
        let dotdot = tar(
            "dotdot.tar.gz",
            &["--transform", "s,^Tool-1/proton,Tool-1/../escaped,", "Tool-1"],
        );
        let script = format!("{COMPAT_DIR_SH}\n{COMPAT_INSTALL_SH}");
        let run_in = |home: &std::path::Path, url: &str, sum: &str, need_kb: &str| {
            Command::new("sh")
                .args(["-c", &script, "sh", url, sum, "Tool-1", need_kb])
                .env("HOME", home)
                .env("LC_ALL", "C")
                .output()
                .unwrap()
        };
        let run = |url: &str| run_in(&home, url, "", "0");
        let tools = home.join(".local/share/Steam/compatibilitytools.d");

        // Checksum: a wrong one is refused, the right one installs (each in a fresh HOME).
        let digest = Command::new("sha512sum")
            .arg(root.join("good.tar.gz"))
            .output()
            .unwrap();
        let digest = String::from_utf8_lossy(&digest.stdout)
            .split(' ')
            .next()
            .unwrap_or("")
            .to_string();
        std::fs::write(root.join("right.sha512sum"), format!("{digest}  good.tar.gz\n")).unwrap();
        std::fs::write(
            root.join("wrong.sha512sum"),
            format!("{}  good.tar.gz\n", "0".repeat(128)),
        )
        .unwrap();
        let sum_url = |n: &str| format!("file://{}", root.join(n).display());
        let home2 = root.join("home2");
        let bad = run_in(&home2, &good, &sum_url("wrong.sha512sum"), "0");
        assert!(!bad.status.success());
        assert!(String::from_utf8_lossy(&bad.stdout).contains("checksum does not match"));
        assert!(!home2.join(".local/share/Steam/compatibilitytools.d/Tool-1").exists());
        let right = run_in(&home2, &good, &sum_url("right.sha512sum"), "0");
        assert!(right.status.success(), "{}", String::from_utf8_lossy(&right.stdout));

        // Free space: needing more than any disk has is refused before downloading.
        let home3 = root.join("home3");
        let full = run_in(&home3, &good, "", "999999999999");
        assert!(!full.status.success());
        let msg = String::from_utf8_lossy(&full.stdout).to_string();
        assert!(msg.contains("not enough free space"), "{msg}");
        assert!(friendly_error(msg.trim()).starts_with("Not enough free space on the device (need"));

        assert!(
            !run(&outside).status.success(),
            "archive with a second top folder must be refused"
        );
        assert!(!tools.join("Other").exists());
        assert!(!run(&dotdot).status.success(), "archive with .. must be refused");
        assert!(!tools.join("escaped").exists());
        let ok = run(&good);
        assert!(ok.status.success(), "{}", String::from_utf8_lossy(&ok.stdout));
        assert!(tools.join("Tool-1/proton").exists());
        let again = run(&good);
        assert!(!again.status.success(), "an existing version must not be overwritten");
        assert!(String::from_utf8_lossy(&again.stdout).contains("already installed"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn errors_become_actionable() {
        assert!(friendly_error("error: Could not resolve host: dl.flathub.org").starts_with("No internet connection"));
        assert!(
            friendly_error("curl: (6) Couldn't resolve host 'api.github.com'").starts_with("No internet connection")
        );
        assert!(friendly_error("error: checksum does not match").contains("damaged"));
        assert!(friendly_error("error: No space left on device").starts_with("Not enough free space"));
        assert!(
            friendly_error("curl: (22) The requested URL returned error: 403 (api.github.com rate limit)")
                .starts_with("GitHub's hourly limit")
        );
        assert_eq!(friendly_error("error: something odd"), "something odd");
    }

    #[test]
    fn simulated_appimage_lifecycle() {
        let (tx, rx) = start(Kind::Simulated { reason: String::new() }, None);
        let cmds = parse_commands(
            "install-appimage\tappimage:sample.emu\tSample AppImage\thttps://github.com/sample/emu/releases/download/v1.0/S.AppImage\t\t0\tS.AppImage\t100\tv1.0\tsample/emu\n",
        );
        assert!(
            matches!(&cmds[0], Cmd::Install { job: Job::AppImage { file, size: 100, .. }, .. } if file == "S.AppImage")
        );
        for c in cmds {
            tx.send(c).unwrap();
        }
        let installed = |want: &dyn Fn(&str) -> bool| loop {
            if let Event::Installed(j) = rx.recv_timeout(Duration::from_secs(5)).unwrap() {
                if want(&j) {
                    break j;
                }
            }
        };
        let j = installed(&|j| j.contains("appimage:sample.emu"));
        assert!(j.contains("\"registered\":\"false\""), "saved, not runnable: {j}");
        tx.send(Cmd::Register("appimage:sample.emu".into())).unwrap();
        installed(&|j| j.contains("\"registered\":\"true\""));
        tx.send(Cmd::Launch("appimage:sample.emu".into())).unwrap();
        tx.send(Cmd::Uninstall("appimage:sample.emu".into())).unwrap();
        installed(&|j| !j.contains("appimage:sample.emu"));
    }

    #[test]
    fn network_commands_parse() {
        let cmds = parse_commands("check-network\nsim-offline\t1\nsim-offline\t0\n");
        assert!(matches!(cmds[0], Cmd::CheckNetwork));
        assert!(matches!(cmds[1], Cmd::SimOffline(true)));
        assert!(matches!(cmds[2], Cmd::SimOffline(false)));
    }

    #[test]
    fn simulated_offline_fails_installs_then_recovers() {
        let (tx, rx) = start(Kind::Simulated { reason: String::new() }, None);
        tx.send(Cmd::SimOffline(true)).unwrap();
        let wait_for = |want: &str| loop {
            if let Event::Network(st, _) = rx.recv_timeout(Duration::from_secs(5)).unwrap() {
                if st == want {
                    break;
                }
            }
        };
        wait_for("offline");
        tx.send(Cmd::Install {
            id: "x.y.Z".into(),
            name: "Z".into(),
            job: Job::Flatpak { update: false },
        })
        .unwrap();
        loop {
            if let Event::Done(_, r) = rx.recv_timeout(Duration::from_secs(5)).unwrap() {
                assert!(r.starts_with("No internet connection"), "{r}");
                break;
            }
        }
        tx.send(Cmd::SimOffline(false)).unwrap();
        wait_for("online");
    }

    #[test]
    fn compat_install_command_parses() {
        let cmds = parse_commands("install-compat\tcompat:GE-Proton11-12\tGE-Proton11-12\thttps://github.com/x/y/releases/download/t/a.tar.gz\thttps://github.com/x/y/releases/download/t/a.sha512sum\n");
        assert!(
            matches!(&cmds[0], Cmd::Install { id, job: Job::Compat { url, sum, .. }, .. }
            if id == "compat:GE-Proton11-12" && url.ends_with(".tar.gz") && sum.ends_with(".sha512sum"))
        );
    }

    #[test]
    fn simulated_releases_look_like_github() {
        let j = sim_releases_json();
        assert_eq!(j.matches("\"tag_name\"").count(), 2);
        assert!(j.contains("browser_download_url"));
    }

    #[test]
    fn commands_parse() {
        let cmds = parse_commands("install\torg.a.B\tB app\ncancel\torg.a.B\nbogus\tx\nsearch\t\nuninstall\torg.c.D\n");
        assert_eq!(cmds.len(), 3);
        assert!(
            matches!(&cmds[0], Cmd::Install { id, name, job: Job::Flatpak { update: false } } if id == "org.a.B" && name == "B app")
        );
    }

    #[test]
    fn simulated_install_and_uninstall() {
        let (tx, rx) = start(Kind::Simulated { reason: String::new() }, None);
        tx.send(Cmd::Install {
            id: "x.y.Z".into(),
            name: "Z".into(),
            job: Job::Flatpak { update: false },
        })
        .unwrap();
        let mut done = false;
        let mut installed = String::new();
        for _ in 0..200 {
            match rx.recv_timeout(Duration::from_secs(5)).unwrap() {
                Event::Done(id, r) => {
                    assert_eq!((id.as_str(), r.as_str()), ("x.y.Z", "ok"));
                    done = true;
                }
                Event::Installed(j) if done => {
                    installed = j;
                    break;
                }
                _ => {}
            }
        }
        assert!(installed.contains("x.y.Z"));
        tx.send(Cmd::Uninstall("x.y.Z".into())).unwrap();
        loop {
            if let Event::Installed(j) = rx.recv_timeout(Duration::from_secs(5)).unwrap() {
                if !j.contains("x.y.Z") {
                    break;
                }
            }
        }
    }

    #[test]
    fn curated_list_is_valid_json_shape() {
        let j = curated_json();
        assert!(j.starts_with('[') && j.ends_with(']'));
        assert_eq!(j.matches("\"id\"").count(), CURATED.len());
    }
}
