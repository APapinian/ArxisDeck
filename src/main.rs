// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! Arxis Deck: a Steam Deck store for software and game tools.
//!
//! A 1280x800 Qt Quick window navigated entirely by controller (or keyboard/touch).
//! Apps come from Flathub and are installed for the current user with `flatpak --user`
//! (no password, survives SteamOS updates). Without Flatpak, and in the automated
//! modes, a simulated store with sample items is used instead.
//!
//! Options:
//!   --screenshot <file.png>   draw ~120 frames, save the window picture, exit
//!   --self-test <file.json>   drive the UI with a scripted gamepad, save results, exit
//!   --fullscreen / --windowed override the automatic choice (full screen in Gaming Mode)
//!   --open <search|menu|details|settings|downloads|installed|categories|history|install|about|offline> [--query <text>]
//!                             start with a view open (for screenshots of that view)
//!   --setting <key=value>     with --screenshot: draw with this setting (e.g. textsize=Largest)
//!   --simulated               use the simulated store even if Flatpak is available
//!   --flatpak-probe           print what the Flatpak backend sees (read-only) and exit

mod appimage;
mod details;
mod diagnostics;
mod github;
mod history;
mod input;
mod json;
mod logging;
mod qt;
mod selftest;
mod settings;
mod store;

use std::sync::mpsc;
use std::time::Instant;

/// The interface: every file under qml/, bundled by build.rs (rcc) and served as qrc:/qml/.
static QML_RCC: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/qml.rcc"));
const TICK_MS: i32 = 16;
const SCREENSHOT_FRAMES: u64 = 120;
const OPEN_VIEW_TICK: u64 = 50;

enum Mode {
    Normal,
    Screenshot(String),
    SelfTest(String),
}

fn usage() -> ! {
    eprintln!(
        "usage: arxis-deck [--fullscreen | --windowed] [--screenshot <file.png>] [--self-test <file.json>]\n\
         \x20                 [--open <search|menu|details|settings|downloads|installed|categories|history|install|about|offline>] [--query <text>]\n\
         \x20                 [--setting <key=value>] [--simulated] [--flatpak-probe]"
    );
    std::process::exit(2);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut mode = Mode::Normal;
    let mut fullscreen: Option<bool> = None;
    let mut open_view: Option<&'static [&'static str]> = None;
    let mut query = String::new();
    let mut start_offline = false;
    let mut force_simulated = false;
    let mut shot_settings: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" => {
                mode = Mode::Screenshot(args.get(i + 1).cloned().unwrap_or_else(|| usage()));
                i += 1;
            }
            "--setting" => {
                shot_settings.push(args.get(i + 1).cloned().unwrap_or_else(|| usage()));
                i += 1;
            }
            "--self-test" => {
                mode = Mode::SelfTest(args.get(i + 1).cloned().unwrap_or_else(|| usage()));
                i += 1;
            }
            "--open" => {
                open_view = match args.get(i + 1).map(String::as_str) {
                    Some("search") => Some(&["search"][..]),
                    Some("menu") => Some(&["menu"][..]),
                    Some("details") => Some(&["confirm"][..]),
                    Some("settings") => Some(&["tab_next", "tab_next", "tab_next", "tab_next"][..]),
                    Some("categories") => Some(&["tab_next", "tab_next", "down"][..]),
                    Some("installed") => Some(&["secondary", "right", "secondary", "tab_next"][..]),
                    Some("history") => Some(&["secondary", "right", "secondary", "view", "menu"][..]),
                    Some("install") => Some(&["right", "secondary"][..]),
                    // Simulated store goes offline first (see OPEN_VIEW_TICK); shows Discover offline.
                    Some("offline") => {
                        start_offline = true;
                        Some(&["right"][..])
                    }
                    Some("about") => {
                        Some(&["tab_next", "tab_next", "tab_next", "tab_next", "page_down", "confirm"][..])
                    }
                    Some("downloads") => Some(
                        &[
                            "secondary",
                            "right",
                            "secondary",
                            "right",
                            "secondary",
                            "down",
                            "secondary",
                            "view",
                        ][..],
                    ),
                    _ => usage(),
                };
                i += 1;
            }
            "--query" => {
                query = args.get(i + 1).cloned().unwrap_or_else(|| usage());
                i += 1;
            }
            "--simulated" => force_simulated = true,
            "--flatpak-probe" => probe(),
            "--flatpak-roundtrip" => roundtrip(args.get(i + 1).cloned().unwrap_or_else(|| usage())),
            "--fullscreen" => fullscreen = Some(true),
            "--windowed" => fullscreen = Some(false),
            "-h" | "--help" => usage(),
            _ => {}
        }
        i += 1;
    }
    let automated = !matches!(mode, Mode::Normal);
    // Log file only in normal runs; the automated modes log to stderr.
    if let Some(path) = logging::init(!automated) {
        tracing::info!(version = env!("CARGO_PKG_VERSION"), log = %path.display(), "Arxis Deck starting");
    }
    // The --open shortcuts press buttons (X adds downloads), so outside the automated
    // modes they always run against the simulated store.
    if open_view.is_some() && !automated {
        force_simulated = true;
    }

    // Gaming Mode runs apps inside gamescope; SteamOS also sets SteamDeck=1.
    let gaming_mode =
        std::env::var_os("GAMESCOPE_WAYLAND_DISPLAY").is_some() || std::env::var("SteamDeck").is_ok_and(|v| v == "1");
    let fullscreen = fullscreen.unwrap_or(gaming_mode && !automated);

    // The automated modes render off screen at exactly 1280x800 and scale 1, so the
    // result does not depend on the desktop's window manager or display scaling.
    // Set QT_QPA_PLATFORM to test them in a real window instead.
    if automated && std::env::var_os("QT_QPA_PLATFORM").is_none() {
        std::env::set_var("QT_QPA_PLATFORM", "offscreen");
        std::env::set_var("QT_QUICK_BACKEND", "software");
        std::env::set_var("QT_SCALE_FACTOR", "1");
    }

    // Build-time Qt QML folders, used only if they exist on this machine.
    let imports: Vec<&str> = env!("ARXIS_QML_IMPORTS")
        .split(':')
        .filter(|d| !d.is_empty() && std::path::Path::new(d).is_dir())
        .collect();

    // Qt only receives the program name; our options are handled above.
    qt::init(&args[..1]);
    if !qt::register_resource(QML_RCC) {
        tracing::error!("could not register the bundled interface");
        std::process::exit(1);
    }
    let base_url = "qrc:/qml/Main.qml";
    if let Err(code) = qt::load("", base_url, &imports.join(":"), fullscreen) {
        tracing::error!("could not load the interface (code {code})");
        std::process::exit(1);
    }

    // Store backend: real Flathub installs in normal runs, simulated in the automated
    // modes (so checks never change the system) or when Flatpak is missing.
    // ARXIS_REAL_STORE=1 lets a screenshot show the real Flatpak catalogue (read-only
    // unless something is installed); the self-test always stays simulated.
    let real_store_shot =
        matches!(mode, Mode::Screenshot(_)) && std::env::var("ARXIS_REAL_STORE").is_ok_and(|v| v == "1");
    let kind = store::detect(if automated && !real_store_shot {
        Some("")
    } else if force_simulated {
        Some("Simulated store (--simulated) · nothing is installed")
    } else {
        None
    });
    qt::set_property("backendStatus", &kind.status());
    if matches!(kind, store::Kind::Flatpak { .. }) {
        qt::set_property("catalogJson", &store::curated_json());
    }
    qt::set_property("backendMode", kind.mode());
    qt::set_property("gamingMode", if gaming_mode { "true" } else { "false" });
    // Operation history: the real database in normal runs, in memory in the automated modes.
    let history = match &mode {
        Mode::Normal => match history::History::open(history::default_path().as_deref()) {
            Ok(h) => Some(h),
            Err(e) => {
                tracing::warn!("operation history unavailable: {e}");
                None
            }
        },
        Mode::SelfTest(_) | Mode::Screenshot(_) => history::History::open(None).ok(),
    };
    // For the About & diagnostics page (gap 4).
    let diag_mode = kind.mode().to_string();
    let diag_status = kind.status();
    let diag_history = if matches!(mode, Mode::Normal) {
        history::default_path()
    } else {
        None
    };
    let diag_settings = if matches!(mode, Mode::Normal) {
        settings::default_path()
    } else {
        None
    };
    // Reports go next to the user's data in normal runs, to the temp folder in test runs.
    let diag_export_dir = if matches!(mode, Mode::Normal) {
        logging::log_dir()
            .and_then(|d| d.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(std::env::temp_dir)
    } else {
        std::env::temp_dir()
    };
    let (backend, backend_events) = store::start(kind, history);

    let (tx, rx) = mpsc::channel::<input::InputMsg>();
    if !automated {
        input::spawn_gamepads(tx);
    } else {
        drop(tx);
    }

    let mut nav = input::Navigator::new();
    let mut pads: Vec<String> = Vec::new();
    let mut unreadable: Vec<String> = Vec::new();
    let mut status_dirty = true;
    let mut ticks: u64 = 0;
    // Settings are saved only in normal runs; the self-test uses a throw-away file and
    // screenshots always use the defaults.
    let selftest_settings = std::env::temp_dir().join(format!("arxis-deck-selftest-{}.toml", std::process::id()));
    let mut store = settings::Store::new(match &mode {
        Mode::Normal => settings::default_path(),
        Mode::SelfTest(_) => Some(selftest_settings.clone()),
        Mode::Screenshot(_) => None,
    });
    store.load_into_ui();
    // Screenshots normally use the defaults; --setting picks others (nothing is saved).
    if matches!(mode, Mode::Screenshot(_)) && !shot_settings.is_empty() {
        qt::set_property("savedSettings", &shot_settings.join("\n"));
    }
    let mut selftest = match &mode {
        Mode::SelfTest(path) => Some(selftest::SelfTest::new(path.clone(), selftest_settings)),
        _ => None,
    };

    qt::set_tick(TICK_MS, move || {
        ticks += 1;
        let now = Instant::now();

        // Optional start-up view, applied once the boot splash has finished.
        if start_offline && ticks == OPEN_VIEW_TICK - 30 {
            qt::set_property("commands", "sim-offline\t1\n");
        }
        if ticks == OPEN_VIEW_TICK {
            if !query.is_empty() {
                qt::set_property("searchQuery", &query.to_lowercase());
            }
            for a in open_view.unwrap_or_default() {
                qt::action(a, "start-up option");
            }
        }

        if ticks.is_multiple_of(settings::POLL_TICKS) {
            if let Some((delay, interval)) = store.sync() {
                nav.set_timing(delay, interval);
            }
            nav.set_swap_xy(store.swap_xy());
        }

        while let Ok(msg) = rx.try_recv() {
            match msg {
                input::InputMsg::Button(ev) => {
                    if let Some(a) = nav.on_event(ev, now) {
                        qt::action(a, "controller");
                    }
                }
                input::InputMsg::Connected(name) => {
                    pads.push(name);
                    status_dirty = true;
                }
                input::InputMsg::Disconnected(name) => {
                    if let Some(p) = pads.iter().position(|n| *n == name) {
                        pads.remove(p);
                    }
                    status_dirty = true;
                }
                input::InputMsg::Unreadable(name) => {
                    unreadable.push(name);
                    status_dirty = true;
                }
            }
        }

        // UI → store commands, store → UI events.
        let commands = qt::property("commands");
        if !commands.is_empty() {
            qt::set_property("commands", "");
            // Diagnostics are answered here: controllers and the window live on this thread.
            for verb in commands
                .lines()
                .filter(|l| l.starts_with("diagnostics") || l.starts_with("export-diagnostics"))
            {
                let ctx = diagnostics::Context {
                    backend_mode: diag_mode.clone(),
                    backend_status: diag_status.clone(),
                    controllers: pads.clone(),
                    window: qt::window_size(),
                    qt_version: qt::qt_version(),
                    history_path: diag_history.clone(),
                    settings_path: diag_settings.clone(),
                };
                let sections = diagnostics::collect(&ctx);
                if verb.starts_with("export") {
                    let (path, err) = match diagnostics::export(&sections, &diag_export_dir) {
                        Ok(p) => (
                            logging::redact(&p.display().to_string(), &std::env::var("HOME").unwrap_or_default()),
                            String::new(),
                        ),
                        Err(e) => (String::new(), e),
                    };
                    qt::invoke3("backendEvent", "exported", &path, &err);
                } else {
                    qt::invoke3("backendEvent", "diagnostics", &diagnostics::to_json(&sections), "");
                }
            }
            // Steam's on-screen keyboard: the automated modes never call Steam.
            if commands.lines().any(|l| l == "steam-keyboard") {
                let (state, err) = if automated {
                    ("test", String::new())
                } else {
                    match store::open_steam_keyboard() {
                        Ok(()) => ("opened", String::new()),
                        Err(e) => ("", e),
                    }
                };
                tracing::info!(state, err = %err, "Steam keyboard requested");
                qt::invoke3("backendEvent", "steam-keyboard", state, &err);
            }
            for cmd in store::parse_commands(&commands) {
                // A real-catalogue screenshot must never change the system: only search passes.
                if real_store_shot && !matches!(cmd, store::Cmd::Search(_) | store::Cmd::Details(_)) {
                    continue;
                }
                let _ = backend.send(cmd);
            }
        }
        while let Ok(ev) = backend_events.try_recv() {
            let (kind, a, b) = ev.to_qml();
            qt::invoke3("backendEvent", kind, &a, &b);
        }

        let mut done = false;
        if let Some(st) = selftest.as_mut() {
            done = st.step(&mut nav, now);
        }
        for a in nav.tick(now) {
            if let Some(st) = selftest.as_mut() {
                st.note_repeat(a);
            }
            qt::action(
                a,
                if selftest.is_some() {
                    "self-test pad"
                } else {
                    "controller"
                },
            );
        }

        if status_dirty {
            status_dirty = false;
            let status = if selftest.is_some() {
                "Controller: scripted self-test pad".to_string()
            } else if let Some(name) = pads.first() {
                let layout = if input::uses_position_layout(name) {
                    " · PlayStation button layout"
                } else {
                    ""
                };
                format!("Controller: {name}{layout}")
            } else if let Some(name) = unreadable.first() {
                format!("Controller \"{name}\" found but not readable (no permission)")
            } else {
                "No controller found · keyboard: arrows, Enter, Esc, Q/E".to_string()
            };
            qt::set_property("controllerStatus", &status);
            qt::set_property(
                "controllerConnected",
                if pads.is_empty() && selftest.is_none() {
                    "false"
                } else {
                    "true"
                },
            );
        }

        match &mode {
            Mode::Screenshot(path) => {
                // Wait for ~120 drawn frames (or ~4 s if frames are not reported).
                if qt::frames() >= SCREENSHOT_FRAMES || ticks >= 240 {
                    if let Some(dir) = std::path::Path::new(path).parent() {
                        let _ = std::fs::create_dir_all(dir);
                    }
                    match qt::grab(path) {
                        Ok(colours) => {
                            eprintln!(
                                "screenshot saved to {path} ({} frames, {colours} colours)",
                                qt::frames()
                            );
                            qt::quit(0);
                        }
                        Err(()) => {
                            eprintln!("could not save screenshot to {path}");
                            qt::quit(1);
                        }
                    }
                }
            }
            Mode::SelfTest(_) => {
                if done {
                    qt::quit(0);
                } else if ticks > 5000 {
                    eprintln!("self-test timed out");
                    qt::quit(1);
                }
            }
            Mode::Normal => {}
        }
    });

    std::process::exit(qt::exec());
}

/// `--flatpak-roundtrip <app-id>`: really installs one Flathub app for this user through the
/// store backend, prints every progress value, checks it is listed, uninstalls it and checks
/// it is gone. Changes the system while it runs; meant for validating a device.
fn roundtrip(id: String) -> ! {
    use std::time::{Duration, Instant};
    let kind = store::detect(None);
    println!("backend: {} ({})", kind.mode(), kind.status());
    if !matches!(kind, store::Kind::Flatpak { .. }) {
        std::process::exit(1);
    }
    if store::installed_json().is_ok_and(|j| j.contains(&format!("\"{}\"", json::escape(&id)))) {
        println!("{id} is already installed for this user; pick an app that is not installed");
        std::process::exit(1);
    }
    let history = history::History::open(history::default_path().as_deref()).ok();
    let (tx, rx) = store::start(kind, history);
    let t0 = Instant::now();
    let _ = tx.send(store::Cmd::Install {
        id: id.clone(),
        name: id.clone(),
        job: store::Job::Flatpak { update: false },
    });
    let mut result = String::new();
    let mut progress = Vec::new();
    let mut listed = false;
    while t0.elapsed() < Duration::from_secs(1800) {
        match rx.recv_timeout(Duration::from_secs(60)) {
            Ok(store::Event::Progress(i, p)) if i == id => {
                progress.push(p);
                println!("  {:>6.1} s  progress {:>5.1} %", t0.elapsed().as_secs_f32(), p * 100.0);
            }
            Ok(store::Event::Done(i, r)) if i == id => {
                println!("  {:>6.1} s  done: {r}", t0.elapsed().as_secs_f32());
                result = r;
                if result != "ok" {
                    break;
                }
            }
            Ok(store::Event::Installed(j)) if result == "ok" => {
                listed = j.contains(&format!("\"{}\"", json::escape(&id)));
                break;
            }
            Ok(_) => {}
            Err(_) => println!("  {:>6.1} s  (still waiting)", t0.elapsed().as_secs_f32()),
        }
    }
    println!(
        "install: {result}; {} progress updates; listed as installed: {listed}",
        progress.len()
    );
    let mut gone = false;
    if listed {
        let _ = tx.send(store::Cmd::Uninstall(id.clone()));
        while let Ok(ev) = rx.recv_timeout(Duration::from_secs(300)) {
            match ev {
                store::Event::Removed(_, r) => println!("uninstall: {r}"),
                store::Event::Installed(j) => {
                    gone = !j.contains(&format!("\"{}\"", json::escape(&id)));
                    break;
                }
                _ => {}
            }
        }
        println!("gone after uninstall: {gone}");
    }
    std::process::exit(if result == "ok" && listed && gone { 0 } else { 1 });
}

/// `--flatpak-probe`: shows what the store would see on this machine, without changing it.
fn probe() -> ! {
    let kind = store::detect(None);
    println!("backend: {} ({})", kind.mode(), kind.status());
    if let store::Kind::Flatpak { .. } = kind {
        match store::installed_json() {
            Ok(j) => println!("user-installed apps: {}", j.matches("\"id\"").count()),
            Err(e) => println!("user-installed apps: error: {e}"),
        }
        match store::search_json("protonup") {
            Ok(j) => println!("search \"protonup\": {} Flathub result(s)", j.matches("\"id\"").count()),
            Err(e) => println!("search: error: {e}"),
        }
        println!("starter list: {} apps", store::CURATED.len());
        match store::ge_releases_json() {
            Ok(j) => println!("Proton-GE releases on GitHub: {}", j.matches("\"tag_name\"").count()),
            Err(e) => println!("Proton-GE releases: error: {e}"),
        }
        println!("Steam compatibility tools installed: {:?}", store::compat_tools());
        for (repo, name, category, summary) in appimage::CURATED {
            let url = format!("https://api.github.com/repos/{repo}/releases/latest");
            let j = std::process::Command::new("curl")
                .args(["-fsSL", "--max-time", "20", &url])
                .output();
            match j.map(|o| String::from_utf8_lossy(&o.stdout).into_owned()) {
                Ok(j) => match appimage::item_from_release(&j, repo, name, category, summary) {
                    Ok(it) => println!(
                        "AppImage {name}: {} {} · {} MB · SHA-256 {}",
                        it.version,
                        it.file,
                        it.size / 1_048_576,
                        if it.sum.is_empty() { "missing" } else { "from GitHub" }
                    ),
                    Err(e) => println!("AppImage {name}: {e}"),
                },
                Err(e) => println!("AppImage {name}: {e}"),
            }
        }
        let d = store::flathub_details("com.heroicgameslauncher.hgl");
        println!(
            "details of Heroic: version {} · download {} · installed {} · {} permissions ({} need attention){}",
            d.version,
            d.download,
            d.installed,
            d.permissions.len(),
            d.attention,
            if d.error.is_empty() {
                String::new()
            } else {
                format!(" · error: {}", d.error)
            }
        );
    }
    std::process::exit(0);
}
