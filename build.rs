// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! Finds Qt 6 (Core, Gui, Qml, Quick), compiles the C++ bridge and links it.
//!
//! Qt is located through `qmake6` (or `$QMAKE`). Extra Qt install prefixes can be
//! given in `$ARXIS_QT_PREFIXES` (colon separated). On Nix, where Qt Quick lives in
//! its own store path, the matching `qtdeclarative` path is found automatically.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn qmake_query(qmake: &str, key: &str) -> Option<String> {
    let out = Command::new(qmake).args(["-query", key]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// A Qt install prefix: where headers, libraries and QML modules live.
struct Prefix {
    include: PathBuf,
    lib: PathBuf,
    qml: PathBuf,
}

impl Prefix {
    fn from_root(root: &Path) -> Prefix {
        Prefix {
            include: root.join("include"),
            lib: root.join("lib"),
            qml: root.join("lib/qt-6/qml"),
        }
    }
}

fn nix_store_matches(suffix: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if let Ok(entries) = fs::read_dir("/nix/store") {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(suffix) && e.path().is_dir() {
                found.push(e.path());
            }
        }
    }
    found.sort();
    found
}

fn main() {
    println!("cargo:rerun-if-changed=src/qt/bridge.cpp");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=QMAKE");
    println!("cargo:rerun-if-env-changed=ARXIS_QT_PREFIXES");

    let qmake = env::var("QMAKE").ok().unwrap_or_else(|| {
        for cand in ["qmake6", "qmake-qt6", "qmake"] {
            if qmake_query(cand, "QT_VERSION").is_some() {
                return cand.to_string();
            }
        }
        panic!("Qt 6 not found: install Qt 6 (qtbase + qtdeclarative) or set QMAKE");
    });
    let version = qmake_query(&qmake, "QT_VERSION").expect("qmake did not report QT_VERSION");
    assert!(version.starts_with("6."), "Qt 6 is required, found Qt {version}");

    let mut prefixes = vec![Prefix {
        include: PathBuf::from(qmake_query(&qmake, "QT_INSTALL_HEADERS").expect("QT_INSTALL_HEADERS")),
        lib: PathBuf::from(qmake_query(&qmake, "QT_INSTALL_LIBS").expect("QT_INSTALL_LIBS")),
        qml: PathBuf::from(qmake_query(&qmake, "QT_INSTALL_QML").unwrap_or_default()),
    }];
    if let Ok(extra) = env::var("ARXIS_QT_PREFIXES") {
        for p in extra.split(':').filter(|p| !p.is_empty()) {
            prefixes.push(Prefix::from_root(Path::new(p)));
        }
    }
    let has_quick = |ps: &[Prefix]| ps.iter().any(|p| p.include.join("QtQuick/QQuickWindow").exists());
    if !has_quick(&prefixes) {
        // Nix keeps Qt Quick in a separate store path from qtbase.
        for root in nix_store_matches(&format!("-qtdeclarative-{version}")) {
            if root.join("include/QtQuick/QQuickWindow").exists() {
                prefixes.push(Prefix::from_root(&root));
                break;
            }
        }
    }
    assert!(
        has_quick(&prefixes),
        "Qt Quick headers not found (install qtdeclarative / qt6-declarative-dev)"
    );

    // Compile the bridge with the system C++ compiler into a static library.
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cxx = env::var("CXX").unwrap_or_else(|_| "c++".to_string());
    let obj = out_dir.join("bridge.o");
    let mut cmd = Command::new(&cxx);
    cmd.args(["-std=c++17", "-fPIC", "-O2", "-c", "src/qt/bridge.cpp", "-o"])
        .arg(&obj);
    for p in &prefixes {
        cmd.arg(format!("-I{}", p.include.display()));
        for module in ["QtCore", "QtGui", "QtQml", "QtQuick"] {
            let dir = p.include.join(module);
            if dir.exists() {
                cmd.arg(format!("-I{}", dir.display()));
            }
        }
    }
    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("could not run C++ compiler '{cxx}': {e}"));
    assert!(status.success(), "compiling src/qt/bridge.cpp failed");
    let lib = out_dir.join("libarxis_qt_bridge.a");
    let _ = fs::remove_file(&lib);
    let status = Command::new("ar")
        .arg("crs")
        .arg(&lib)
        .arg(&obj)
        .status()
        .expect("could not run ar");
    assert!(status.success(), "ar failed");

    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=arxis_qt_bridge");
    for p in &prefixes {
        if p.lib.exists() {
            println!("cargo:rustc-link-search=native={}", p.lib.display());
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", p.lib.display());
        }
    }
    for l in ["Qt6Quick", "Qt6Qml", "Qt6Gui", "Qt6Core", "stdc++"] {
        println!("cargo:rustc-link-lib=dylib={l}");
    }

    // QML module folders found at build time, used as a fallback import path at runtime.
    let qml_dirs: Vec<String> = prefixes
        .iter()
        .filter(|p| p.qml.join("QtQuick").exists())
        .map(|p| p.qml.display().to_string())
        .collect();
    println!("cargo:rustc-env=ARXIS_QML_IMPORTS={}", qml_dirs.join(":"));

    // Bundle every file under qml/ into a Qt resource (qrc:/qml/...), so the program carries
    // its whole interface (ARCHITECTURE.md §3.2 layout: pages/, components/, ...).
    println!("cargo:rerun-if-changed=qml");
    let libexec = qmake_query(&qmake, "QT_HOST_LIBEXECS").unwrap_or_default();
    let rcc = [PathBuf::from(&libexec).join("rcc"), PathBuf::from("rcc")]
        .into_iter()
        .find(|p| {
            Command::new(p)
                .arg("--version")
                .output()
                .is_ok_and(|o| o.status.success())
        })
        .expect("Qt's rcc was not found (QT_HOST_LIBEXECS)");
    let mut files = Vec::new();
    let mut stack = vec![PathBuf::from("qml")];
    while let Some(dir) = stack.pop() {
        for e in fs::read_dir(&dir).expect("read qml/").flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "qml" || x == "js" || x == "svg") {
                files.push(p);
            }
        }
    }
    files.sort();
    let root = env::current_dir().unwrap();
    let mut qrc = String::from("<RCC>\n  <qresource prefix=\"/\">\n");
    for f in &files {
        qrc.push_str(&format!(
            "    <file alias=\"{}\">{}</file>\n",
            f.display(),
            root.join(f).display()
        ));
    }
    qrc.push_str("  </qresource>\n</RCC>\n");
    let qrc_path = out_dir.join("qml.qrc");
    fs::write(&qrc_path, qrc).unwrap();
    let status = Command::new(&rcc)
        .args(["--binary", "-o"])
        .arg(out_dir.join("qml.rcc"))
        .arg(&qrc_path)
        .status()
        .expect("could not run rcc");
    assert!(status.success(), "rcc failed");
}
