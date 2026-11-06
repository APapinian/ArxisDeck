// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! Safe wrappers over the C++ bridge in `src/qt/bridge.cpp`.
//! All functions must be called on the Qt GUI thread (i.e. from `main` or the tick callback).

use std::ffi::{c_char, c_int, c_void, CStr, CString};

extern "C" {
    fn arx_init(argc: c_int, argv: *mut *mut c_char) -> c_int;
    fn arx_load(
        qml: *const c_char,
        len: usize,
        base_url: *const c_char,
        imports: *const c_char,
        fullscreen: c_int,
    ) -> c_int;
    fn arx_set_tick(cb: extern "C" fn(*mut c_void), user: *mut c_void, interval_ms: c_int);
    fn arx_action(name: *const c_char, source: *const c_char);
    fn arx_invoke3(method: *const c_char, a: *const c_char, b: *const c_char, c: *const c_char);
    fn arx_set_property(name: *const c_char, value: *const c_char);
    fn arx_get_property(name: *const c_char, buf: *mut c_char, cap: c_int) -> c_int;
    fn arx_grab(path: *const c_char) -> c_int;
    fn arx_frames() -> u64;
    fn arx_qt_version() -> *const c_char;
    fn arx_register_resource(data: *const u8) -> c_int;
    fn arx_window_size(w: *mut c_int, h: *mut c_int) -> c_int;
    fn arx_quit(code: c_int);
    fn arx_exec() -> c_int;
}

fn cstr(s: &str) -> CString {
    CString::new(s.replace('\0', "")).unwrap()
}

pub fn init(args: &[String]) {
    // Qt keeps pointers to argc/argv for the whole run, so they are leaked on purpose.
    let mut ptrs: Vec<*mut c_char> = args.iter().map(|a| cstr(a).into_raw()).collect();
    ptrs.push(std::ptr::null_mut());
    let argv = Box::leak(ptrs.into_boxed_slice());
    unsafe { arx_init(args.len() as c_int, argv.as_mut_ptr()) };
}

pub fn load(qml: &str, base_url: &str, imports: &str, fullscreen: bool) -> Result<(), i32> {
    let base = cstr(base_url);
    let imp = cstr(imports);
    let rc = unsafe {
        arx_load(
            qml.as_ptr() as *const c_char,
            qml.len(),
            base.as_ptr(),
            imp.as_ptr(),
            fullscreen as c_int,
        )
    };
    if rc == 0 {
        Ok(())
    } else {
        Err(rc)
    }
}

/// Runs `f` every `interval_ms` on the GUI thread until the app exits.
pub fn set_tick<F: FnMut() + 'static>(interval_ms: i32, f: F) {
    extern "C" fn trampoline(user: *mut c_void) {
        let f = unsafe { &mut *(user as *mut Box<dyn FnMut()>) };
        f();
    }
    let boxed: Box<Box<dyn FnMut()>> = Box::new(Box::new(f));
    unsafe { arx_set_tick(trampoline, Box::into_raw(boxed) as *mut c_void, interval_ms) };
}

pub fn action(name: &str, source: &str) {
    let (n, s) = (cstr(name), cstr(source));
    unsafe { arx_action(n.as_ptr(), s.as_ptr()) };
}

/// Calls the QML root function `method(a, b, c)`.
pub fn invoke3(method: &str, a: &str, b: &str, c: &str) {
    let (m, a, b, c) = (cstr(method), cstr(a), cstr(b), cstr(c));
    unsafe { arx_invoke3(m.as_ptr(), a.as_ptr(), b.as_ptr(), c.as_ptr()) };
}

pub fn set_property(name: &str, value: &str) {
    let (n, v) = (cstr(name), cstr(value));
    unsafe { arx_set_property(n.as_ptr(), v.as_ptr()) };
}

pub fn property(name: &str) -> String {
    let n = cstr(name);
    let mut buf = vec![0 as c_char; 4096];
    let rc = unsafe { arx_get_property(n.as_ptr(), buf.as_mut_ptr(), buf.len() as c_int) };
    if rc < 0 {
        return String::new();
    }
    unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned()
}

pub fn property_i64(name: &str) -> i64 {
    property(name).trim().parse::<f64>().map(|v| v as i64).unwrap_or(-1)
}

pub fn property_bool(name: &str) -> bool {
    property(name).trim() == "true"
}

/// Saves the window as PNG; returns how many distinct colours were sampled.
pub fn grab(path: &str) -> Result<i32, ()> {
    let p = cstr(path);
    let rc = unsafe { arx_grab(p.as_ptr()) };
    if rc < 0 {
        Err(())
    } else {
        Ok(rc)
    }
}

/// Registers the bundled QML (made by rcc in build.rs); `data` lives for the whole run.
pub fn register_resource(data: &'static [u8]) -> bool {
    unsafe { arx_register_resource(data.as_ptr()) == 0 }
}

/// Qt version in use at run time.
pub fn qt_version() -> String {
    unsafe { CStr::from_ptr(arx_qt_version()) }
        .to_string_lossy()
        .into_owned()
}

pub fn frames() -> u64 {
    unsafe { arx_frames() }
}

pub fn window_size() -> (i32, i32) {
    let (mut w, mut h) = (0, 0);
    unsafe { arx_window_size(&mut w, &mut h) };
    (w, h)
}

pub fn quit(code: i32) {
    unsafe { arx_quit(code) };
}

pub fn exec() -> i32 {
    unsafe { arx_exec() }
}
