// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! Operation history in SQLite (ROADMAP.md M1 task 4, M2 task 5; PROJECT.md §5.3).
//!
//! `~/.local/share/arxis-deck/arxis-deck.sqlite` (ARCHITECTURE.md §8). SQLite here is a record
//! of what Arxis Deck did, never the authority on what is installed: that always comes from
//! the providers. Schema changes go through numbered migrations.

use rusqlite::{params, Connection};
use serde::Serialize;
use std::path::PathBuf;

/// Migrations, applied in order; the index + 1 is the schema version. Never edit an old one.
const MIGRATIONS: &[&str] = &[
    // 1: operations
    "CREATE TABLE operations (
        id          INTEGER PRIMARY KEY,
        started_at  TEXT NOT NULL,
        finished_at TEXT,
        kind        TEXT NOT NULL,
        target_id   TEXT NOT NULL,
        target_name TEXT NOT NULL,
        source      TEXT NOT NULL,
        result      TEXT
     );
     CREATE INDEX operations_started ON operations(started_at);",
];

const NOW: &str = "strftime('%Y-%m-%dT%H:%M:%SZ', 'now')";

#[derive(Serialize, Debug, PartialEq)]
pub struct Operation {
    pub id: i64,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub kind: String,
    pub target_id: String,
    pub target_name: String,
    pub source: String,
    pub result: Option<String>,
}

pub struct History {
    db: Connection,
}

pub fn default_path() -> Option<PathBuf> {
    crate::logging::log_dir().and_then(|d| d.parent().map(|p| p.join("arxis-deck.sqlite")))
}

impl History {
    /// Opens (or creates) the database and brings its schema up to date.
    pub fn open(path: Option<&std::path::Path>) -> Result<History, String> {
        let db = match path {
            Some(p) => {
                if let Some(dir) = p.parent() {
                    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
                }
                Connection::open(p)
            }
            None => Connection::open_in_memory(),
        }
        .map_err(|e| e.to_string())?;
        let mut h = History { db };
        h.migrate()?;
        h.reconcile_interrupted()?;
        Ok(h)
    }

    pub fn schema_version(&self) -> i64 {
        self.db.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap_or(0)
    }

    fn migrate(&mut self) -> Result<(), String> {
        let current = self.schema_version() as usize;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(current) {
            let tx = self.db.transaction().map_err(|e| e.to_string())?;
            tx.execute_batch(sql).map_err(|e| format!("migration {}: {e}", i + 1))?;
            tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))
                .map_err(|e| e.to_string())?;
            tx.commit().map_err(|e| e.to_string())?;
            tracing::info!(version = i + 1, "history database migrated");
        }
        Ok(())
    }

    /// Operations left unfinished by a previous run (app closed, crash) are marked "interrupted".
    fn reconcile_interrupted(&self) -> Result<(), String> {
        let n = self
            .db
            .execute(
                &format!("UPDATE operations SET finished_at = {NOW}, result = 'interrupted' WHERE finished_at IS NULL"),
                [],
            )
            .map_err(|e| e.to_string())?;
        if n > 0 {
            tracing::warn!(count = n, "operations from an earlier run were interrupted");
        }
        Ok(())
    }

    pub fn start(&self, kind: &str, target_id: &str, target_name: &str, source: &str) -> Option<i64> {
        let r = self.db.execute(
            &format!(
                "INSERT INTO operations (started_at, kind, target_id, target_name, source) VALUES ({NOW}, ?1, ?2, ?3, ?4)"
            ),
            params![kind, target_id, target_name, source],
        );
        match r {
            Ok(_) => Some(self.db.last_insert_rowid()),
            Err(e) => {
                tracing::warn!("history: {e}");
                None
            }
        }
    }

    pub fn finish(&self, op: Option<i64>, result: &str) {
        let Some(op) = op else { return };
        if let Err(e) = self.db.execute(
            &format!("UPDATE operations SET finished_at = {NOW}, result = ?1 WHERE id = ?2"),
            params![result, op],
        ) {
            tracing::warn!("history: {e}");
        }
    }

    /// One-step record (used by the tests).
    #[cfg(test)]
    pub fn record(&self, kind: &str, target_id: &str, target_name: &str, source: &str, result: &str) {
        let op = self.start(kind, target_id, target_name, source);
        self.finish(op, result);
    }

    /// Newest first.
    pub fn recent(&self, limit: usize) -> Vec<Operation> {
        let Ok(mut st) = self.db.prepare(
            "SELECT id, started_at, finished_at, kind, target_id, target_name, source, result
             FROM operations ORDER BY id DESC LIMIT ?1",
        ) else {
            return Vec::new();
        };
        st.query_map([limit as i64], |r| {
            Ok(Operation {
                id: r.get(0)?,
                started_at: r.get(1)?,
                finished_at: r.get(2)?,
                kind: r.get(3)?,
                target_id: r.get(4)?,
                target_name: r.get(5)?,
                source: r.get(6)?,
                result: r.get(7)?,
            })
        })
        .map(|rows| rows.filter_map(Result::ok).collect())
        .unwrap_or_default()
    }

    pub fn recent_json(&self, limit: usize) -> String {
        serde_json::to_string(&self.recent(limit)).unwrap_or_else(|_| "[]".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_once_and_records_operations() {
        let h = History::open(None).unwrap();
        assert_eq!(h.schema_version(), MIGRATIONS.len() as i64);
        let op = h.start("install", "org.a.B", "B", "flatpak");
        assert!(op.is_some());
        h.finish(op, "ok");
        h.record("launch", "org.a.B", "B", "flatpak", "ok");
        let ops = h.recent(10);
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].kind, "launch", "newest first");
        assert_eq!(ops[1].result.as_deref(), Some("ok"));
        assert!(ops[1].started_at.ends_with('Z'));
    }

    #[test]
    fn reopening_keeps_data_and_marks_interrupted_work() {
        let path = std::env::temp_dir().join(format!("arxis-history-test-{}.sqlite", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let h = History::open(Some(&path)).unwrap();
            h.record("install", "org.a.B", "B", "flatpak", "ok");
            h.start("update", "org.a.B", "B", "flatpak"); // never finished: app "closed"
        }
        let h = History::open(Some(&path)).unwrap();
        assert_eq!(h.schema_version(), MIGRATIONS.len() as i64, "no migration runs twice");
        let ops = h.recent(10);
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].result.as_deref(), Some("interrupted"));
        let _ = std::fs::remove_file(&path);
    }
}
