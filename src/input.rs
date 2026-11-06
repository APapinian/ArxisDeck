// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! Controller input without external crates.
//!
//! Pipeline: raw Linux evdev events (`/dev/input/event*`) → [`Mapper`] (per device:
//! buttons, D-pad, hat, stick and trigger thresholds) → [`ButtonEvent`] → channel →
//! [`Navigator`] on the GUI thread (hold-to-repeat) → action names for QML.
//!
//! In Gaming Mode, Steam Input exposes the Deck controls as a virtual Xbox-style pad,
//! which shows up here as an ordinary evdev gamepad.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// Values from mechanics.json ("navigation" group).
pub const REPEAT_DELAY_MS: u64 = 400;
pub const REPEAT_INTERVAL_MS: u64 = 110;
pub const AXIS_PRESS: f32 = 0.6;
pub const AXIS_RELEASE: f32 = 0.35;
pub const RESCAN_MS: u64 = 2000;

// linux/input-event-codes.h
pub const EV_KEY: u16 = 0x01;
pub const EV_ABS: u16 = 0x03;
pub const BTN_SOUTH: u16 = 0x130;
pub const BTN_EAST: u16 = 0x131;
pub const BTN_NORTH: u16 = 0x133;
pub const BTN_WEST: u16 = 0x134;
pub const BTN_TL: u16 = 0x136;
pub const BTN_TR: u16 = 0x137;
pub const BTN_TL2: u16 = 0x138;
pub const BTN_TR2: u16 = 0x139;
pub const BTN_SELECT: u16 = 0x13a;
pub const BTN_START: u16 = 0x13b;
pub const BTN_DPAD_UP: u16 = 0x220;
pub const BTN_DPAD_DOWN: u16 = 0x221;
pub const BTN_DPAD_LEFT: u16 = 0x222;
pub const BTN_DPAD_RIGHT: u16 = 0x223;
pub const ABS_X: u16 = 0x00;
pub const ABS_Y: u16 = 0x01;
pub const ABS_Z: u16 = 0x02;
pub const ABS_RZ: u16 = 0x05;
pub const ABS_HAT0X: u16 = 0x10;
pub const ABS_HAT0Y: u16 = 0x11;
pub const ABS_HAT2X: u16 = 0x14;
pub const ABS_HAT2Y: u16 = 0x15;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Button {
    Up,
    Down,
    Left,
    Right,
    A,
    B,
    X,
    Y,
    L1,
    R1,
    L2,
    R2,
    Start,
    Select,
}

impl Button {
    /// The navigation action this button triggers (see the button map in MECHANICS.md).
    pub fn action(self) -> &'static str {
        match self {
            Button::Up => "up",
            Button::Down => "down",
            Button::Left => "left",
            Button::Right => "right",
            Button::A => "confirm",
            Button::B => "back",
            Button::X => "search",    // ARCHITECTURE.md §3.3: X opens search
            Button::Y => "secondary", // Y: secondary actions
            Button::L1 => "tab_prev",
            Button::R1 => "tab_next",
            Button::L2 => "page_up",
            Button::R2 => "page_down",
            Button::Start => "menu",
            Button::Select => "view",
        }
    }

    fn repeats(self) -> bool {
        matches!(
            self,
            Button::Up | Button::Down | Button::Left | Button::Right | Button::L2 | Button::R2
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonEvent {
    pub button: Button,
    pub pressed: bool,
}

pub enum InputMsg {
    Button(ButtonEvent),
    Connected(String),
    Disconnected(String),
    Unreadable(String),
}

/// Controllers whose driver names the face buttons by position (Linux gamepad rule):
/// the left button sends BTN_WEST and the top one BTN_NORTH. Xbox-style pads (xpad, and
/// Steam Input's virtual pad in Gaming Mode) send the printed label instead: X → BTN_X,
/// which is the same code as BTN_NORTH. PlayStation pads used without Steam Input follow
/// the position rule, so their two codes are swapped relative to Xbox pads.
pub fn uses_position_layout(name: &str) -> bool {
    let n = name.to_lowercase();
    [
        "sony",
        "playstation",
        "dualsense",
        "dualshock",
        "ps4 ",
        "ps5 ",
        "wireless controller",
    ]
    .iter()
    .any(|k| n.contains(k))
}

fn key_button(code: u16, position_layout: bool) -> Option<Button> {
    // Left face button → X, top → Y, whichever code the driver uses for them.
    if position_layout {
        match code {
            BTN_WEST => return Some(Button::X),
            BTN_NORTH => return Some(Button::Y),
            _ => {}
        }
    }
    Some(match code {
        BTN_SOUTH => Button::A,
        BTN_EAST => Button::B,
        BTN_NORTH => Button::X,
        BTN_WEST => Button::Y,
        BTN_TL => Button::L1,
        BTN_TR => Button::R1,
        BTN_TL2 => Button::L2,
        BTN_TR2 => Button::R2,
        BTN_SELECT => Button::Select,
        BTN_START => Button::Start,
        BTN_DPAD_UP => Button::Up,
        BTN_DPAD_DOWN => Button::Down,
        BTN_DPAD_LEFT => Button::Left,
        BTN_DPAD_RIGHT => Button::Right,
        _ => return None,
    })
}

/// Turns one device's raw events into button presses/releases.
/// A button pressed from several sources (e.g. D-pad and stick) is only reported once.
#[derive(Default)]
pub struct Mapper {
    /// See [`uses_position_layout`].
    pub position_layout: bool,
    ranges: HashMap<u16, (i32, i32)>,
    sources: HashMap<Button, HashSet<u32>>,
}

impl Mapper {
    pub fn new() -> Mapper {
        Mapper::default()
    }

    pub fn set_range(&mut self, code: u16, min: i32, max: i32) {
        if max > min {
            self.ranges.insert(code, (min, max));
        }
    }

    fn range(&self, code: u16) -> (i32, i32) {
        if let Some(r) = self.ranges.get(&code) {
            return *r;
        }
        match code {
            ABS_X | ABS_Y => (-32768, 32767),
            ABS_Z | ABS_RZ => (0, 255),
            ABS_HAT2X | ABS_HAT2Y => (0, 32767),
            _ => (-1, 1),
        }
    }

    fn set(&mut self, button: Button, source: u32, on: bool, out: &mut Vec<ButtonEvent>) {
        let set = self.sources.entry(button).or_default();
        let was = !set.is_empty();
        if on {
            set.insert(source);
        } else {
            set.remove(&source);
        }
        let now = !set.is_empty();
        if was != now {
            out.push(ButtonEvent { button, pressed: now });
        }
    }

    /// Axis with a centre (stick or hat): drives `neg`/`pos` buttons with hysteresis.
    fn centred_axis(&mut self, code: u16, value: i32, neg: Button, pos: Button, out: &mut Vec<ButtonEvent>) {
        let (min, max) = self.range(code);
        let mid = (min as f32 + max as f32) / 2.0;
        let half = (max as f32 - min as f32) / 2.0;
        let v = if half > 0.0 { (value as f32 - mid) / half } else { 0.0 };
        let src = 0x1_0000 | code as u32;
        for (button, signed) in [(neg, -v), (pos, v)] {
            let held = self.sources.get(&button).is_some_and(|s| s.contains(&src));
            let on = if held {
                signed > AXIS_RELEASE
            } else {
                signed > AXIS_PRESS
            };
            self.set(button, src, on, out);
        }
    }

    /// One-sided axis (analog trigger).
    fn trigger_axis(&mut self, code: u16, value: i32, button: Button, out: &mut Vec<ButtonEvent>) {
        let (min, max) = self.range(code);
        let v = (value - min) as f32 / (max - min).max(1) as f32;
        let src = 0x1_0000 | code as u32;
        let held = self.sources.get(&button).is_some_and(|s| s.contains(&src));
        let on = if held { v > AXIS_RELEASE } else { v > AXIS_PRESS };
        self.set(button, src, on, out);
    }

    pub fn feed(&mut self, ty: u16, code: u16, value: i32, out: &mut Vec<ButtonEvent>) {
        match ty {
            EV_KEY => {
                if value == 2 {
                    return; // kernel autorepeat; the Navigator does its own repeat
                }
                if let Some(b) = key_button(code, self.position_layout) {
                    self.set(b, code as u32, value != 0, out);
                }
            }
            EV_ABS => match code {
                ABS_HAT0X => self.centred_axis(code, value, Button::Left, Button::Right, out),
                ABS_HAT0Y => self.centred_axis(code, value, Button::Up, Button::Down, out),
                ABS_X => self.centred_axis(code, value, Button::Left, Button::Right, out),
                ABS_Y => self.centred_axis(code, value, Button::Up, Button::Down, out),
                ABS_Z | ABS_HAT2Y => self.trigger_axis(code, value, Button::L2, out),
                ABS_RZ | ABS_HAT2X => self.trigger_axis(code, value, Button::R2, out),
                _ => {}
            },
            _ => {}
        }
    }
}

/// Turns presses into actions, repeating held directions like Steam's own UI.
pub struct Navigator {
    held: Vec<(Button, Instant)>,
    delay_ms: u64,
    interval_ms: u64,
    swap_xy: bool,
}

impl Navigator {
    pub fn new() -> Navigator {
        Navigator {
            held: Vec::new(),
            delay_ms: REPEAT_DELAY_MS,
            interval_ms: REPEAT_INTERVAL_MS,
            swap_xy: false,
        }
    }

    /// Hold-to-repeat timing (Settings → Hold-to-repeat speed).
    pub fn set_timing(&mut self, delay_ms: u64, interval_ms: u64) {
        self.delay_ms = delay_ms.clamp(100, 2000);
        self.interval_ms = interval_ms.clamp(30, 1000);
    }

    pub fn timing(&self) -> (u64, u64) {
        (self.delay_ms, self.interval_ms)
    }

    /// Settings → "X and Y buttons: Swapped", for controllers that report them the other way round.
    pub fn set_swap_xy(&mut self, swap: bool) {
        self.swap_xy = swap;
    }

    pub fn on_event(&mut self, mut ev: ButtonEvent, now: Instant) -> Option<&'static str> {
        if self.swap_xy {
            ev.button = match ev.button {
                Button::X => Button::Y,
                Button::Y => Button::X,
                b => b,
            };
        }
        self.held.retain(|(b, _)| *b != ev.button);
        if ev.pressed {
            if ev.button.repeats() {
                self.held.push((ev.button, now + Duration::from_millis(self.delay_ms)));
            }
            Some(ev.button.action())
        } else {
            None
        }
    }

    pub fn tick(&mut self, now: Instant) -> Vec<&'static str> {
        let mut out = Vec::new();
        for (b, next) in self.held.iter_mut() {
            while *next <= now {
                out.push(b.action());
                *next += Duration::from_millis(self.interval_ms);
            }
        }
        out
    }
}

// ---- Device discovery and reading ----

extern "C" {
    fn ioctl(fd: i32, request: u64, ...) -> i32;
}

#[repr(C)]
#[derive(Default)]
struct AbsInfo {
    value: i32,
    minimum: i32,
    maximum: i32,
    fuzz: i32,
    flat: i32,
    resolution: i32,
}

fn eviocgabs(abs: u16) -> u64 {
    // _IOR('E', 0x40 + abs, struct input_absinfo)
    (2u64 << 30) | ((std::mem::size_of::<AbsInfo>() as u64) << 16) | ((b'E' as u64) << 8) | (0x40 + abs as u64)
}

/// True if the sysfs key-capability bitmask (hex words, most significant first) has `bit`.
pub fn has_key_bit(caps: &str, bit: u32) -> bool {
    let words: Vec<&str> = caps.split_whitespace().collect();
    let idx = (bit / 64) as usize;
    if idx >= words.len() {
        return false;
    }
    let word = words[words.len() - 1 - idx];
    u64::from_str_radix(word, 16)
        .map(|w| w >> (bit % 64) & 1 == 1)
        .unwrap_or(false)
}

fn device_name(event: &str) -> String {
    fs::read_to_string(format!("/sys/class/input/{event}/device/name"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| event.to_string())
}

fn is_gamepad(event: &str) -> bool {
    fs::read_to_string(format!("/sys/class/input/{event}/device/capabilities/key"))
        .map(|caps| has_key_bit(&caps, BTN_SOUTH as u32))
        .unwrap_or(false)
}

fn read_device(path: PathBuf, name: String, mut file: File, tx: Sender<InputMsg>, open: Arc<Mutex<HashSet<PathBuf>>>) {
    let mut mapper = Mapper::new();
    mapper.position_layout = uses_position_layout(&name);
    let fd = file.as_raw_fd();
    for code in [ABS_X, ABS_Y, ABS_Z, ABS_RZ, ABS_HAT0X, ABS_HAT0Y, ABS_HAT2X, ABS_HAT2Y] {
        let mut info = AbsInfo::default();
        if unsafe { ioctl(fd, eviocgabs(code), &mut info as *mut AbsInfo) } == 0 {
            mapper.set_range(code, info.minimum, info.maximum);
        }
    }
    let _ = tx.send(InputMsg::Connected(name.clone()));
    let mut buf = [0u8; 24 * 64];
    let mut out = Vec::new();
    loop {
        let n = match file.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for ev in buf[..n - n % 24].as_chunks::<24>().0 {
            let ty = u16::from_ne_bytes([ev[16], ev[17]]);
            let code = u16::from_ne_bytes([ev[18], ev[19]]);
            let value = i32::from_ne_bytes([ev[20], ev[21], ev[22], ev[23]]);
            mapper.feed(ty, code, value, &mut out);
        }
        for e in out.drain(..) {
            if tx.send(InputMsg::Button(e)).is_err() {
                return;
            }
        }
    }
    open.lock().unwrap().remove(&path);
    let _ = tx.send(InputMsg::Disconnected(name));
}

/// Watches `/dev/input` for gamepads (including hot-plugged ones) in the background.
pub fn spawn_gamepads(tx: Sender<InputMsg>) {
    thread::spawn(move || {
        let open: Arc<Mutex<HashSet<PathBuf>>> = Arc::new(Mutex::new(HashSet::new()));
        let mut reported_unreadable: HashSet<PathBuf> = HashSet::new();
        loop {
            if let Ok(entries) = fs::read_dir("/dev/input") {
                for e in entries.flatten() {
                    let event = e.file_name().to_string_lossy().to_string();
                    let path = e.path();
                    if !event.starts_with("event") || open.lock().unwrap().contains(&path) || !is_gamepad(&event) {
                        continue;
                    }
                    let name = device_name(&event);
                    match File::open(&path) {
                        Ok(file) => {
                            open.lock().unwrap().insert(path.clone());
                            let (tx2, open2) = (tx.clone(), open.clone());
                            thread::spawn(move || read_device(path, name, file, tx2, open2));
                        }
                        Err(_) => {
                            if reported_unreadable.insert(path) && tx.send(InputMsg::Unreadable(name)).is_err() {
                                return;
                            }
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(RESCAN_MS));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(m: &mut Mapper, ty: u16, code: u16, value: i32) -> Vec<ButtonEvent> {
        let mut out = Vec::new();
        m.feed(ty, code, value, &mut out);
        out
    }

    #[test]
    fn buttons_map_to_actions() {
        let mut m = Mapper::new();
        let ev = feed(&mut m, EV_KEY, BTN_SOUTH, 1);
        assert_eq!(
            ev,
            vec![ButtonEvent {
                button: Button::A,
                pressed: true
            }]
        );
        assert_eq!(ev[0].button.action(), "confirm");
        assert!(feed(&mut m, EV_KEY, BTN_SOUTH, 2).is_empty());
        assert!(!feed(&mut m, EV_KEY, BTN_SOUTH, 0)[0].pressed);
    }

    #[test]
    fn stick_has_hysteresis_and_merges_with_dpad() {
        let mut m = Mapper::new();
        assert_eq!(
            feed(&mut m, EV_ABS, ABS_X, 30000)[0],
            ButtonEvent {
                button: Button::Right,
                pressed: true
            }
        );
        assert!(feed(&mut m, EV_ABS, ABS_X, 15000).is_empty()); // still above release threshold
        assert!(feed(&mut m, EV_KEY, BTN_DPAD_RIGHT, 1).is_empty()); // already held via stick
        assert!(feed(&mut m, EV_ABS, ABS_X, 0).is_empty()); // D-pad still holds it
        assert!(!feed(&mut m, EV_KEY, BTN_DPAD_RIGHT, 0)[0].pressed);
    }

    #[test]
    fn trigger_presses_page() {
        let mut m = Mapper::new();
        assert_eq!(feed(&mut m, EV_ABS, ABS_RZ, 255)[0].button.action(), "page_down");
    }

    #[test]
    fn held_direction_repeats() {
        let mut n = Navigator::new();
        let t0 = Instant::now();
        assert_eq!(
            n.on_event(
                ButtonEvent {
                    button: Button::Down,
                    pressed: true
                },
                t0
            ),
            Some("down")
        );
        assert!(n.tick(t0 + Duration::from_millis(REPEAT_DELAY_MS - 1)).is_empty());
        let reps = n.tick(t0 + Duration::from_millis(REPEAT_DELAY_MS + 2 * REPEAT_INTERVAL_MS));
        assert_eq!(reps.len(), 3);
        n.on_event(
            ButtonEvent {
                button: Button::Down,
                pressed: false,
            },
            t0,
        );
        assert!(n.tick(t0 + Duration::from_secs(5)).is_empty());
    }

    #[test]
    fn repeat_timing_can_change() {
        let mut n = Navigator::new();
        n.set_timing(300, 70);
        let t0 = Instant::now();
        n.on_event(
            ButtonEvent {
                button: Button::Right,
                pressed: true,
            },
            t0,
        );
        assert_eq!(n.tick(t0 + Duration::from_millis(300 + 70)).len(), 2);
    }

    #[test]
    fn x_and_y_follow_the_controller_convention() {
        // Xbox-style / Steam virtual pad: printed labels.
        let mut xbox = Mapper::new();
        assert_eq!(feed(&mut xbox, EV_KEY, BTN_NORTH, 1)[0].button.action(), "search");
        assert_eq!(feed(&mut xbox, EV_KEY, BTN_WEST, 1)[0].button.action(), "secondary");
        // PlayStation pad without Steam Input: positions.
        let mut ps = Mapper::new();
        ps.position_layout = uses_position_layout("Sony Interactive Entertainment DualSense Wireless Controller");
        assert!(ps.position_layout);
        assert_eq!(feed(&mut ps, EV_KEY, BTN_WEST, 1)[0].button.action(), "search");
        assert_eq!(feed(&mut ps, EV_KEY, BTN_NORTH, 1)[0].button.action(), "secondary");
        assert!(!uses_position_layout("Microsoft X-Box 360 pad 0"));
        assert!(!uses_position_layout("Steam Deck"));
    }

    #[test]
    fn manual_swap_exchanges_x_and_y() {
        let mut n = Navigator::new();
        let t0 = Instant::now();
        n.set_swap_xy(true);
        assert_eq!(
            n.on_event(
                ButtonEvent {
                    button: Button::X,
                    pressed: true
                },
                t0
            ),
            Some("secondary")
        );
        assert_eq!(
            n.on_event(
                ButtonEvent {
                    button: Button::A,
                    pressed: true
                },
                t0
            ),
            Some("confirm")
        );
    }

    #[test]
    fn capability_bits() {
        // bit 0x130 = 304 → word 4 (from the end), bit 48
        assert!(has_key_bit("7fdb000000000000 0 0 0 0", 304));
        assert!(!has_key_bit("0 0 0 0 0", 304));
        assert!(!has_key_bit("ffff", 304));
    }
}
