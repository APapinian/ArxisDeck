// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! Structured logging with `tracing` (ARCHITECTURE.md §12).
//!
//! Every line goes to stderr and, in normal runs, to
//! `$XDG_DATA_HOME/arxis-deck/logs/arxis-deck.log` (ARCHITECTURE.md §8). The home folder is
//! replaced by `~` in both, so logs can be shared without the user's name. The file is
//! rotated to `arxis-deck.log.1` when it is larger than [`MAX_LOG_BYTES`] at start-up.
//! `ARXIS_LOG=debug` (or `trace`, `warn`, `error`) changes the level; the default is `info`.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

pub const MAX_LOG_BYTES: u64 = 1_000_000;

pub fn log_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(base.join("arxis-deck").join("logs"))
}

/// Replaces the home folder with `~` (ARCHITECTURE.md §9: redact personal paths).
pub fn redact(text: &str, home: &str) -> String {
    if home.len() > 1 {
        text.replace(home, "~")
    } else {
        text.to_string()
    }
}

#[derive(Clone)]
struct Sink {
    file: Option<Arc<Mutex<File>>>,
    home: Arc<String>,
}

struct SinkWriter {
    sink: Sink,
}

impl Write for SinkWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = redact(&String::from_utf8_lossy(buf), &self.sink.home);
        let _ = io::stderr().write_all(text.as_bytes());
        if let Some(f) = &self.sink.file {
            if let Ok(mut f) = f.lock() {
                let _ = f.write_all(text.as_bytes());
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Sink {
    type Writer = SinkWriter;
    fn make_writer(&'a self) -> SinkWriter {
        SinkWriter { sink: self.clone() }
    }
}

fn open_log_file() -> Option<(File, PathBuf)> {
    let dir = log_dir()?;
    fs::create_dir_all(&dir).ok()?;
    let path = dir.join("arxis-deck.log");
    if fs::metadata(&path).map(|m| m.len() > MAX_LOG_BYTES).unwrap_or(false) {
        let _ = fs::rename(&path, dir.join("arxis-deck.log.1"));
    }
    let f = OpenOptions::new().create(true).append(true).open(&path).ok()?;
    Some((f, path))
}

/// Starts logging; `to_file` is false in the automated modes. Returns the log file, if any.
pub fn init(to_file: bool) -> Option<PathBuf> {
    let level = match std::env::var("ARXIS_LOG").unwrap_or_default().to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    let opened = if to_file { open_log_file() } else { None };
    let path = opened.as_ref().map(|(_, p)| p.clone());
    let sink = Sink {
        file: opened.map(|(f, _)| Arc::new(Mutex::new(f))),
        home: Arc::new(std::env::var("HOME").unwrap_or_default()),
    };
    let _ = tracing_subscriber::fmt()
        .with_writer(sink)
        .with_ansi(false)
        .with_target(false)
        .with_max_level(level)
        .try_init();
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_file_goes_under_xdg_data_home_and_rotates() {
        let tmp = std::env::temp_dir().join(format!("arxis-log-test-{}", std::process::id()));
        std::env::set_var("XDG_DATA_HOME", &tmp);
        let dir = log_dir().unwrap();
        assert_eq!(dir, tmp.join("arxis-deck/logs"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("arxis-deck.log"), vec![b'x'; (MAX_LOG_BYTES + 1) as usize]).unwrap();
        let (mut f, path) = open_log_file().unwrap();
        writeln!(f, "hello").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello\n");
        assert!(dir.join("arxis-deck.log.1").exists(), "the big old log is kept as .1");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn home_is_redacted() {
        assert_eq!(
            redact("saved to /home/fatih/.config/arxis-deck/config.toml", "/home/fatih"),
            "saved to ~/.config/arxis-deck/config.toml"
        );
        assert_eq!(redact("no home here", ""), "no home here");
    }
}
