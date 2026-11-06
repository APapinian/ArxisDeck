// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! `--self-test <file.json>`: drives the UI with a scripted virtual gamepad.
//!
//! The script feeds raw evdev events through the same [`Mapper`] and [`Navigator`] a
//! real controller uses, then reads the UI state back from QML to judge each step.

use crate::input::*;
use crate::qt;
use std::collections::HashMap;
use std::time::Instant;

enum Op {
    Raw(u16, u16, i32),
    Snap(&'static str),
    /// Holds the script until no download is active (at most this many ticks).
    WaitIdle(u32),
    /// A line for the QML `commands` property (e.g. the simulated store's offline switch).
    Cmd(&'static str),
    /// Sets a QML root property (e.g. pretend to run in Gaming Mode).
    Set(&'static str, &'static str),
    Finish,
}

#[derive(Clone, Default)]
struct Snap {
    focus: i64,
    tab: i64,
    columns: i64,
    detail: bool,
    content_y: i64,
    toast: bool,
    hud: bool,
    hud_report: String,
    last: String,
    search_open: bool,
    query: String,
    results: i64,
    menu_open: bool,
    theme: String,
    repeat_timing: String,
    queue_count: i64,
    done_count: i64,
    active_count: i64,
    dl_status: String,
    installed_count: i64,
    category_filter: String,
    last_launched: String,
    update_count: i64,
    installed_json: String,
    confirm_open: bool,
    history_open: bool,
    history_json: String,
    confirm_permissions: i64,
    confirm_text: String,
    about_open: bool,
    diagnostics_json: String,
    export_path: String,
    network: String,
    subtitle: String,
    downloading: i64,
    text_scale: String,
    reduced_motion: bool,
    steam_keyboard: i64,
}

fn snap() -> Snap {
    Snap {
        focus: qt::property_i64("focusIndex"),
        tab: qt::property_i64("tabIndex"),
        columns: qt::property_i64("columns"),
        detail: qt::property_bool("detailOpen"),
        content_y: qt::property_i64("gridContentY"),
        toast: qt::property_bool("toastVisible"),
        hud: qt::property_bool("hudReady"),
        hud_report: qt::property("hudReport"),
        last: qt::property("lastAction"),
        search_open: qt::property_bool("searchOpen"),
        query: qt::property("searchQuery"),
        results: qt::property_i64("resultCount"),
        menu_open: qt::property_bool("menuOpen"),
        theme: qt::property("themeName"),
        repeat_timing: qt::property("repeatTiming"),
        queue_count: qt::property_i64("queueCount"),
        done_count: qt::property_i64("doneCount"),
        active_count: qt::property_i64("activeCount"),
        dl_status: qt::property("selectedDownloadStatus"),
        installed_count: qt::property_i64("installedCount"),
        category_filter: qt::property("categoryFilter"),
        last_launched: qt::property("lastLaunched"),
        update_count: qt::property_i64("updateCount"),
        installed_json: qt::property("installedJson"),
        confirm_open: qt::property_bool("confirmOpen"),
        history_open: qt::property_bool("historyOpen"),
        history_json: qt::property("historyJson"),
        confirm_permissions: qt::property_i64("confirmPermissionCount"),
        confirm_text: qt::property("confirmKind"),
        about_open: qt::property_bool("aboutOpen"),
        diagnostics_json: qt::property("diagnosticsJson"),
        export_path: qt::property("exportPath"),
        network: qt::property("networkState"),
        subtitle: qt::property("headerSubtitle"),
        downloading: qt::property_i64("downloadingCount"),
        text_scale: qt::property("textScale"),
        reduced_motion: qt::property_bool("reducedMotion"),
        steam_keyboard: qt::property_i64("steamKeyboardRequests"),
    }
}

pub struct SelfTest {
    out_path: String,
    tick: u32,
    script: Vec<(u32, Op)>,
    pos: usize,
    pad: Mapper,
    snaps: HashMap<&'static str, Snap>,
    hold_started: Option<Instant>,
    hold_ms: u128,
    sent: Vec<&'static str>,
    settings_path: std::path::PathBuf,
    nav_timing_fast: (u64, u64),
    saved_file: String,
    delay: u32,
    waited: u32,
}

impl SelfTest {
    pub fn new(out_path: String, settings_path: std::path::PathBuf) -> SelfTest {
        let mut s: Vec<(u32, Op)> = Vec::new();
        let mut t = 45; // let the window load and settle
        let tap = |s: &mut Vec<(u32, Op)>, t: &mut u32, ty: u16, code: u16, on: i32, off: i32| {
            s.push((*t, Op::Raw(ty, code, on)));
            s.push((*t + 4, Op::Raw(ty, code, off)));
            *t += 12;
        };
        s.push((t, Op::Snap("start")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0); // D-pad right
        tap(&mut s, &mut t, EV_ABS, ABS_HAT0X, 1, 0); // hat right (Xbox-style pads)
        tap(&mut s, &mut t, EV_ABS, ABS_Y, 32767, 0); // left stick down
        s.push((t, Op::Snap("moved")));
        t += 2;
        s.push((t, Op::Raw(EV_KEY, BTN_DPAD_DOWN, 1))); // hold down → repeat + scroll
        t += 45;
        s.push((t, Op::Raw(EV_KEY, BTN_DPAD_DOWN, 0)));
        t += 14;
        s.push((t, Op::Snap("held")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0); // A
        s.push((t, Op::Snap("detail_open")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_EAST, 1, 0); // B
        s.push((t, Op::Snap("detail_closed")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_TR, 1, 0); // R1
        s.push((t, Op::Snap("tab_next")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_TL, 1, 0); // L1
        s.push((t, Op::Snap("tab_prev")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_WEST, 1, 0); // Y (secondary): install confirmation opens
        s.push((t, Op::Snap("install_confirm")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0); // from Cancel to Install
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0); // A: added to Downloads
        s.push((t, Op::Snap("toast")));
        t += 2;
        // Search: X opens the keyboard (focus on "q"), up to "1", A, right to "2", A, Start = done.
        tap(&mut s, &mut t, EV_KEY, BTN_NORTH, 1, 0); // X (search)
        s.push((t, Op::Snap("search_open")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_UP, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        s.push((t, Op::Snap("typed")));
        t += 2;
        // In Gaming Mode, View asks for Steam's keyboard (the test run does not call Steam).
        s.push((t, Op::Set("gamingMode", "true")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_SELECT, 1, 0);
        t += 4;
        s.push((t, Op::Snap("steam_keyboard")));
        t += 2;
        s.push((t, Op::Set("gamingMode", "false")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_START, 1, 0); // Done
        t += 10;
        s.push((t, Op::Snap("results")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_EAST, 1, 0); // B clears the search
        s.push((t, Op::Snap("cleared")));
        t += 2;
        // Options menu: Start opens, down moves, B closes.
        tap(&mut s, &mut t, EV_KEY, BTN_START, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        s.push((t, Op::Snap("menu_open")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_EAST, 1, 0);
        t += 10;
        s.push((t, Op::Snap("menu_closed")));
        t += 2;
        // Settings: R1 x4, down to repeat speed, right = Fast; up to theme, right = next theme;
        // down x5 to Reset, A; B back to the first section.
        for _ in 0..4 {
            tap(&mut s, &mut t, EV_KEY, BTN_TR, 1, 0);
        }
        s.push((t, Op::Snap("settings")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        t += 24; // let Rust pick up the new timing
        s.push((t, Op::Snap("repeat_fast")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_UP, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        t += 24; // let Rust save the file
        s.push((t, Op::Snap("themed")));
        t += 2;
        // X and Y buttons: down to the row, right = Swapped; then X must act as Y (secondary).
        for _ in 0..5 {
            tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        }
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        t += 24; // let Rust pick up the swap
        tap(&mut s, &mut t, EV_KEY, BTN_NORTH, 1, 0);
        s.push((t, Op::Snap("xy_swapped")));
        t += 2;
        // Text size: right x2 = Largest; Motion: right = Reduced; then past Search keyboard to Reset.
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        t += 4;
        s.push((t, Op::Snap("access")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        s.push((t, Op::Snap("reset")));
        t += 24;
        tap(&mut s, &mut t, EV_KEY, BTN_NORTH, 1, 0); // after Reset, X is X (search) again
        s.push((t, Op::Snap("xy_normal")));
        t += 2;
        // About & diagnostics: down to the last row, A opens it, Y saves a report, B closes it.
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        t += 6;
        s.push((t, Op::Snap("about_open")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_WEST, 1, 0);
        t += 12;
        s.push((t, Op::Snap("about_exported")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_EAST, 1, 0);
        s.push((t, Op::Snap("about_closed")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_EAST, 1, 0);
        t += 30;
        // Downloads (simulated): Y on two cards, Select opens Downloads, down + A pauses,
        // Y takes it out, wait for the rest to finish, B back.
        s.push((t, Op::Snap("dl_start")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_WEST, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_WEST, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        s.push((t, Op::Snap("dl_added")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_SELECT, 1, 0);
        s.push((t, Op::Snap("dl_view")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        s.push((t, Op::Snap("dl_paused")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_WEST, 1, 0);
        s.push((t, Op::Snap("dl_removed")));
        t += 2;
        s.push((t, Op::WaitIdle(1200)));
        t += 2;
        s.push((t, Op::Snap("dl_done")));
        t += 2;
        // Activity history: ≡ opens it (operations recorded in SQLite), B closes it.
        tap(&mut s, &mut t, EV_KEY, BTN_START, 1, 0);
        s.push((t, Op::Snap("history_open")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_EAST, 1, 0);
        s.push((t, Op::Snap("history_closed")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_EAST, 1, 0);
        t += 30;
        // Library: R1 to Installed, A launches, X updates, Y + confirm uninstalls; R1 to Categories, right + A opens
        // "Emulator" in Discover; B returns to Categories; L1 x2 back to Discover.
        tap(&mut s, &mut t, EV_KEY, BTN_TR, 1, 0);
        s.push((t, Op::Snap("installed")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0); // A launches the selected app
        t += 4;
        s.push((t, Op::Snap("launched")));
        t += 2;
        // Updates: down to the app that has one, X updates it, wait until it finishes.
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        s.push((t, Op::Snap("update_ready")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_NORTH, 1, 0);
        s.push((t, Op::Snap("updating")));
        t += 2;
        s.push((t, Op::WaitIdle(1200)));
        t += 20;
        s.push((t, Op::Snap("updated")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_WEST, 1, 0); // Y opens the uninstall confirmation
        s.push((t, Op::Snap("uninstall_armed")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0); // from Cancel to Uninstall
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0); // A confirms
        t += 6;
        s.push((t, Op::Snap("uninstalled")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_TR, 1, 0);
        s.push((t, Op::Snap("categories")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        t += 10;
        s.push((t, Op::Snap("category_open")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_EAST, 1, 0);
        s.push((t, Op::Snap("category_back")));
        t += 2;
        // Proton-GE: right to "Compatibility", A opens it, down to the first Proton-GE card,
        // Y installs it into Steam's compatibility tools; B back to Categories.
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        t += 10;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        s.push((t, Op::Snap("proton_card")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_WEST, 1, 0); // Y opens the install confirmation
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0); // A: Install
        s.push((t, Op::WaitIdle(1200)));
        t += 20;
        s.push((t, Op::Snap("proton_installed")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_EAST, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_TL, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_TL, 1, 0);
        t += 30;
        // Offline: the simulated store loses its connection; an install waits, then finishes by
        // itself once the connection is back.
        s.push((t, Op::Cmd("sim-offline\t1")));
        t += 20;
        s.push((t, Op::Snap("offline")));
        t += 2;
        for _ in 0..7 {
            tap(&mut s, &mut t, EV_KEY, BTN_DPAD_UP, 1, 0);
        }
        tap(&mut s, &mut t, EV_KEY, BTN_WEST, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        t += 20;
        s.push((t, Op::Snap("offline_queued")));
        t += 2;
        s.push((t, Op::Cmd("sim-offline\t0")));
        t += 20;
        s.push((t, Op::WaitIdle(1200)));
        t += 20;
        s.push((t, Op::Snap("back_online")));
        t += 2;
        // AppImage: Categories → Tool → the sample AppImage (5th item) → Y + Install; it is saved but
        // not runnable. In Installed: R2 to the last row, A asks to make it runnable, confirm; then
        // Y + confirm takes it away again. L1 back to Discover.
        tap(&mut s, &mut t, EV_KEY, BTN_TR, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_TR, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_LEFT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_LEFT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        t += 10;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_DOWN, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_WEST, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        s.push((t, Op::WaitIdle(1200)));
        t += 20;
        s.push((t, Op::Snap("appimage_saved")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_EAST, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_TL, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_TR2, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        s.push((t, Op::Snap("appimage_enable")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        t += 12;
        s.push((t, Op::Snap("appimage_runnable")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_WEST, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_DPAD_RIGHT, 1, 0);
        tap(&mut s, &mut t, EV_KEY, BTN_SOUTH, 1, 0);
        t += 12;
        s.push((t, Op::Snap("appimage_gone")));
        t += 2;
        tap(&mut s, &mut t, EV_KEY, BTN_TL, 1, 0);
        t += 30;
        s.push((t, Op::Snap("hud")));
        s.push((t + 2, Op::Finish));
        s.sort_by_key(|(at, _)| *at);
        SelfTest {
            out_path,
            tick: 0,
            script: s,
            pos: 0,
            pad: Mapper::new(),
            snaps: HashMap::new(),
            hold_started: None,
            hold_ms: 0,
            sent: Vec::new(),
            settings_path,
            nav_timing_fast: (0, 0),
            saved_file: String::new(),
            delay: 0,
            waited: 0,
        }
    }

    /// Runs one tick of the script. Returns true when finished (results written).
    pub fn step(&mut self, nav: &mut Navigator, now: Instant) -> bool {
        self.tick += 1;
        while self.pos < self.script.len() && self.script[self.pos].0 + self.delay <= self.tick {
            match self.script[self.pos].1 {
                Op::Raw(ty, code, value) => {
                    // The first D-pad-down press is the long hold that tests repeat.
                    if code == BTN_DPAD_DOWN && self.hold_ms == 0 {
                        if value == 1 {
                            self.hold_started = Some(now);
                        } else if let Some(t0) = self.hold_started {
                            self.hold_ms = now.duration_since(t0).as_millis().max(1);
                        }
                    }
                    let mut evs = Vec::new();
                    self.pad.feed(ty, code, value, &mut evs);
                    for e in evs {
                        if let Some(a) = nav.on_event(e, now) {
                            self.sent.push(a);
                            qt::action(a, "self-test pad");
                        }
                    }
                }
                Op::Snap(name) => {
                    if name == "repeat_fast" {
                        self.nav_timing_fast = nav.timing();
                    }
                    if name == "themed" {
                        self.saved_file = std::fs::read_to_string(&self.settings_path).unwrap_or_default();
                    }
                    self.snaps.insert(name, snap());
                }
                Op::Cmd(line) => {
                    let cur = qt::property("commands");
                    qt::set_property("commands", &format!("{cur}{line}\n"));
                }
                Op::Set(name, value) => qt::set_property(name, value),
                Op::WaitIdle(max) => {
                    if qt::property_i64("activeCount") > 0 && self.waited < max {
                        self.waited += 1;
                        self.delay += 1;
                        break;
                    }
                }
                Op::Finish => {
                    self.pos = self.script.len();
                    self.finish();
                    return true;
                }
            }
            self.pos += 1;
        }
        false
    }

    pub fn note_repeat(&mut self, action: &'static str) {
        self.sent.push(action);
    }

    fn finish(&mut self) {
        let g = |k: &str| self.snaps.get(k).cloned().unwrap_or_default();
        let (start, moved, held) = (g("start"), g("moved"), g("held"));
        let (open, closed, next, prev, toast, hud) = (
            g("detail_open"),
            g("detail_closed"),
            g("tab_next"),
            g("tab_prev"),
            g("toast"),
            g("hud"),
        );
        let cols = moved.columns.max(1);
        let rows_held = (held.focus - moved.focus) / cols;

        let move_ok = moved.focus == start.focus + 2 + cols && rows_held >= 2 && held.content_y > 0;
        let move_detail = format!(
            "D-pad, hat and stick moved focus from card {} to card {}; holding down for {} ms repeated {} rows and scrolled the grid {} px",
            start.focus + 1, moved.focus + 1, self.hold_ms, rows_held, held.content_y
        );

        let iconf = g("install_confirm");
        let confirm_ok = iconf.confirm_open && iconf.confirm_text == "install" && iconf.confirm_permissions > 0;
        let act_ok = open.detail
            && !closed.detail
            && next.tab == start.tab + 1
            && prev.tab == start.tab
            && prev.focus == held.focus
            && confirm_ok
            && toast.toast;
        let act_detail = format!(
            "A opened details: {}; B closed them: {}; R1 went to section {} and L1 back to {} with focus restored to card {}: {}; Y opened the install confirmation with {} permissions listed: {}; confirming showed a notice: {}",
            open.detail, !closed.detail, next.tab + 1, prev.tab + 1, prev.focus + 1, prev.focus == held.focus,
            iconf.confirm_permissions, confirm_ok, toast.toast
        );

        let shot = std::env::temp_dir().join(format!("arxis-deck-selftest-{}.png", std::process::id()));
        let colours = qt::grab(&shot.to_string_lossy()).unwrap_or(0);
        let _ = std::fs::remove_file(&shot);
        let (w, h) = qt::window_size();
        let hud_ok = hud.hud && colours >= 8;
        let hud_detail = format!(
            "top bar with section tabs, card grid, button hint bar and controller status are drawn ({}); window {}x{}, {} colours sampled",
            hud.hud_report, w, h, colours
        );

        let (sopen, typed, results, cleared) = (g("search_open"), g("typed"), g("results"), g("cleared"));
        let search_ok = sopen.search_open
            && typed.query == "12"
            && !results.search_open
            && results.query == "12"
            && results.results == 1
            && cleared.query.is_empty()
            && cleared.results == start.results;
        let search_detail = format!(
            "X opened the keyboard: {}; typed \"{}\" with D-pad and A; Start closed it with {} match(es) for \"{}\"; B cleared it back to {} items",
            sopen.search_open, typed.query, results.results, results.query, cleared.results
        );

        let (mopen, mclosed) = (g("menu_open"), g("menu_closed"));
        let menu_ok = mopen.menu_open && !mclosed.menu_open;
        let menu_detail = format!(
            "Start opened the options menu: {}; B closed it: {}",
            mopen.menu_open, !mclosed.menu_open
        );

        let (sset, fast, themed, reset) = (g("settings"), g("repeat_fast"), g("themed"), g("reset"));
        let (xy_swapped, xy_normal) = (g("xy_swapped"), g("xy_normal"));
        let saved_ok =
            self.saved_file.contains("theme = \"Ocean Depth\"") && self.saved_file.contains("repeat = \"Fast\"");
        let settings_ok = sset.tab == 4
            && fast.repeat_timing == "300,70"
            && self.nav_timing_fast == (300, 70)
            && themed.theme == "Ocean Depth"
            && saved_ok
            && reset.theme == "Arch Midnight"
            && reset.repeat_timing == "400,110"
            && hud.tab == 0
            && xy_swapped.last == "secondary"
            && xy_normal.last == "search";
        let settings_detail = format!(
            "R1 reached Settings: {}; Fast repeat set {} and the controller now repeats at {:?} ms; theme changed to {}; settings file saved: {}; with X/Y swapped the X button did \"{}\"; Reset restored {} / {} and X did \"{}\" again; B returned to section {}",
            sset.tab == 4, fast.repeat_timing, self.nav_timing_fast, themed.theme, saved_ok,
            xy_swapped.last, reset.theme, reset.repeat_timing, xy_normal.last, hud.tab + 1
        );
        let _ = std::fs::remove_file(&self.settings_path);

        let (d0, dadd, dview, dpause, drem, ddone) = (
            g("dl_start"),
            g("dl_added"),
            g("dl_view"),
            g("dl_paused"),
            g("dl_removed"),
            g("dl_done"),
        );
        let (access, skb) = (g("access"), g("steam_keyboard"));
        let access_ok = access.text_scale == "1.3"
            && access.reduced_motion
            && reset.text_scale == "1"
            && !reset.reduced_motion
            && skb.steam_keyboard == typed.steam_keyboard + 1
            && skb.search_open
            && skb.query == typed.query;
        let access_detail = format!(
            "Settings → Text size Largest gave {}× text and Motion Reduced turned motion off: {}; Reset gave {}× and full motion: {}; in Gaming Mode View in search asked for Steam's keyboard ({} request, search still open with \"{}\")",
            access.text_scale, access.reduced_motion, reset.text_scale, !reset.reduced_motion,
            skb.steam_keyboard - typed.steam_keyboard, skb.query
        );
        let (aopen, aexp, aclosed) = (g("about_open"), g("about_exported"), g("about_closed"));
        let report = {
            let p = aexp
                .export_path
                .replacen("~", &std::env::var("HOME").unwrap_or_default(), 1);
            let text = std::fs::read_to_string(&p).unwrap_or_default();
            let _ = std::fs::remove_file(&p);
            text
        };
        let home = std::env::var("HOME").unwrap_or_default();
        let report_ok = report.contains("[Store]")
            && report.contains("[Controller]")
            && (home.len() < 2 || !report.contains(&home));
        let about_ok = aopen.about_open
            && aopen.diagnostics_json.contains("Arxis Deck")
            && aopen.diagnostics_json.contains("Qt")
            && !aexp.export_path.is_empty()
            && report_ok
            && !aclosed.about_open;
        let about_detail = format!(
            "Settings → About & diagnostics opened: {}; it showed version, Qt, system, store, controller and files: {}; Y saved a report ({} bytes) with the home folder hidden: {}; B closed it: {}",
            aopen.about_open, aopen.diagnostics_json.contains("Qt"), report.len(), report_ok, !aclosed.about_open
        );
        let downloads_ok = dadd.queue_count == d0.queue_count + 2
            && dview.tab == 3
            && dview.queue_count == dadd.queue_count
            && dpause.dl_status == "paused"
            && drem.queue_count == dadd.queue_count - 1
            && ddone.active_count == 0
            && ddone.done_count == drem.queue_count
            && hud.tab == 0;
        let downloads_detail = format!(
            "Y added {} items ({} → {}); Select opened Downloads: {}; A paused the second row: {}; Y took it out ({} left); all {} finished their simulated download after waiting {} ticks; B returned to section {}",
            dadd.queue_count - d0.queue_count, d0.queue_count, dadd.queue_count, dview.tab == 3, dpause.dl_status,
            drem.queue_count, ddone.done_count, self.waited, hud.tab + 1
        );

        let (hopen, hclosed) = (g("history_open"), g("history_closed"));
        let installs_logged = hopen.history_json.matches("\"kind\":\"install\"").count();
        let history_ok = hopen.history_open
            && !hclosed.history_open
            && installs_logged >= 2
            && hopen.history_json.contains("\"result\":\"ok\"");
        let history_detail = format!(
            "≡ in Downloads opened the activity history: {}; it listed {} install operations from SQLite with results; B closed it: {}",
            hopen.history_open, installs_logged, !hclosed.history_open
        );

        let (inst, uninst, cats, copen, cback) = (
            g("installed"),
            g("uninstalled"),
            g("categories"),
            g("category_open"),
            g("category_back"),
        );
        let armed = g("uninstall_armed");
        let launched = g("launched");
        let (upd_ready, updating, updated) = (g("update_ready"), g("updating"), g("updated"));
        let updates_ok = upd_ready.update_count == 1
            && updating.active_count == 1
            && updated.update_count == 0
            && updated.installed_count == inst.installed_count;
        let library_ok = inst.tab == 1
            && inst.installed_count == ddone.done_count
            && inst.installed_count > 0
            && !launched.last_launched.is_empty()
            && updates_ok
            && armed.confirm_open
            && armed.installed_count == inst.installed_count
            && !uninst.confirm_open
            && uninst.installed_count == inst.installed_count - 1
            && cats.tab == 2
            && copen.tab == 0
            && copen.category_filter == "Emulator"
            && copen.results == 4
            && cback.tab == 2
            && cback.category_filter.is_empty()
            && hud.tab == 0;
        let library_detail = format!(
            "Installed listed {} apps; A launched \"{}\"; X updated the one with an update ({} → {} updates, {} still installed); Y opened the uninstall confirmation: {} ({} still installed), confirming uninstalled it ({} left); Categories opened \"{}\" in Discover with {} items; B returned to section {}",
            inst.installed_count, launched.last_launched, upd_ready.update_count, updated.update_count, updated.installed_count,
            armed.confirm_open, armed.installed_count, uninst.installed_count, copen.category_filter, copen.results, cback.tab + 1
        );

        let (asaved, aenable, arun, agone) = (
            g("appimage_saved"),
            g("appimage_enable"),
            g("appimage_runnable"),
            g("appimage_gone"),
        );
        let aid = "appimage:sample.emu";
        let saved_ok =
            asaved.installed_json.contains(aid) && asaved.installed_json.contains("\"registered\":\"false\"");
        let runnable_ok = arun.installed_json.contains("\"registered\":\"true\"");
        let enable_asked = aenable.confirm_open && aenable.confirm_text == "enable";
        let gone_ok = !agone.installed_json.contains(aid);
        let appimage_ok = saved_ok && enable_asked && runnable_ok && gone_ok && hud.tab == 0;
        let appimage_detail = format!(
            "Categories → Tool → Sample AppImage → Y + Install: saved, not runnable: {saved_ok}; A in Installed asked to make it runnable: {enable_asked}; confirmed → runnable: {runnable_ok}; Y + confirm took it away: {gone_ok}"
        );
        let (off, offq, back) = (g("offline"), g("offline_queued"), g("back_online"));
        let offline_ok = off.network == "offline"
            && off.subtitle.contains("Offline")
            && offq.active_count >= 1
            && offq.downloading == 0
            && back.network == "online"
            && back.active_count == 0;
        let offline_detail = format!(
            "store went {} (shown: {}); an install queued while offline waited ({} waiting, {} running); back {} it finished by itself ({} still active)",
            off.network, off.subtitle.contains("Offline"), offq.active_count, offq.downloading, back.network, back.active_count
        );
        let (pcard, pinst) = (g("proton_card"), g("proton_installed"));
        let proton_ok = pcard.category_filter == "Compatibility"
            && pinst.installed_json.contains("compat:GE-Proton10-25")
            && pinst.installed_json.contains("\"source\":\"compat\"");
        let proton_detail = format!(
            "Categories → {} → down → Y: GE-Proton10-25 listed as an installed Steam compatibility tool: {}",
            pcard.category_filter,
            pinst.installed_json.contains("compat:GE-Proton10-25")
        );

        let steps = [
            ("move", move_ok, move_detail),
            ("act", act_ok, act_detail),
            ("search", search_ok, search_detail),
            ("menu", menu_ok, menu_detail),
            ("settings", settings_ok, settings_detail),
            ("access", access_ok, access_detail),
            ("about", about_ok, about_detail),
            ("downloads", downloads_ok, downloads_detail),
            ("history", history_ok, history_detail),
            ("library", library_ok, library_detail),
            ("proton", proton_ok, proton_detail),
            ("offline", offline_ok, offline_detail),
            ("appimage", appimage_ok, appimage_detail),
            ("hud", hud_ok, hud_detail),
        ];
        let mut json = String::from("{\"version\": 1, \"steps\": [");
        for (i, (name, ok, detail)) in steps.iter().enumerate() {
            if i > 0 {
                json.push_str(", ");
            }
            json.push_str(&format!(
                "{{\"step\": \"{}\", \"passed\": {}, \"detail\": \"{}\"}}",
                name,
                ok,
                escape(detail)
            ));
        }
        json.push_str(&format!(
            "], \"actions_sent\": {}, \"last_action\": \"{}\"}}\n",
            self.sent.len(),
            escape(&hud.last)
        ));
        if let Some(dir) = std::path::Path::new(&self.out_path).parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match std::fs::write(&self.out_path, json) {
            Ok(()) => eprintln!("self-test written to {}", self.out_path),
            Err(e) => eprintln!("could not write {}: {e}", self.out_path),
        }
    }
}

fn escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}
