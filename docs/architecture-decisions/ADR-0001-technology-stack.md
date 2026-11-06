# ADR-0001: Technology stack for the first builds

- **Status:** accepted by the owner on 2026-11-06: adopt step by step (serde + tracing first, then SQLite)
- **Date:** 2026-11-06

## Context

ARCHITECTURE.md §2 proposes Rust + Qt 6 Quick/QML, with `cxx-qt`, Tokio, SQLite, `reqwest`,
`serde` and `tracing` as candidates to validate in M0/M1. The first instruction for this work
was "do not introduce new dependencies", so the prototype was built without external crates.

## What was built and proven

| Area | ARCHITECTURE.md candidate | Used now | Proven by |
|---|---|---|---|
| UI | Qt 6 Quick/QML | Qt 6.11 Quick/QML | runs native and in the Flatpak sandbox |
| Rust ↔ Qt | `cxx-qt` | ~150-line C++ bridge (`src/qt/bridge.cpp`) compiled by `build.rs`; typed calls both ways (properties, `handleAction`, `backendEvent`) | M0 check "QML receives a typed event from Rust" — passes |
| Async work | Tokio | std threads + channels | install queue, cancel, search, update checks off the UI thread |
| HTTP | `reqwest` | host `curl` (also needed inside the Flatpak sandbox) | Proton-GE releases and downloads |
| Serialization | `serde` | small JSON writer (`src/json.rs`); JSON is parsed in QML | catalogue, installed list, releases |
| Database | SQLite | none yet; installed state always comes from the providers | — |
| Logging | `tracing` | `eprintln!` | — |
| Packaging | AppImage candidate | Flatpak on KDE runtime 6.11 | sandbox self-test, real host access |

## Options

1. **Keep zero external crates** for now. Pro: offline, tiny build, nothing to review. Con: no
   SQLite history, structured logging, typed JSON or robust HTTP; more hand-written code.
2. **Adopt the documented stack step by step** (`serde`/`serde_json`, `tracing`, `rusqlite`
   with bundled SQLite, then `cxx-qt`, Tokio, `reqwest`). Pro: matches ARCHITECTURE.md, needed
   for M2 history and M3 downloads. Con: dependency and licence review per crate; the Flatpak
   build then needs vendored crates (`cargo vendor`) to stay offline.

## Progress

- **Step 1 done (2026-11-06):** `serde`/`serde_json` (GitHub releases parsed into typed structs,
  asset rules in `src/github.rs` with tests; JSON sent to QML built by serde_json) and `tracing`
  (`src/logging.rs`: stderr + `~/.local/share/arxis-deck/logs/arxis-deck.log`, home folder
  shown as `~`, rotated at 1 MB, `ARXIS_LOG=debug`). Crates vendored in `vendor/` with
  `.cargo/config.toml`; the Flatpak build stays offline.
- **Step 2 done (2026-11-06):** `rusqlite` with bundled SQLite (`src/history.rs`):
  `~/.local/share/arxis-deck/arxis-deck.sqlite`, numbered migrations via `PRAGMA user_version`,
  operation history with interruption reconciliation. Installed state still comes from providers.
- **Next:** decide `cxx-qt` versus the current bridge (measured); Tokio/`reqwest` when the
  AppImage/GitHub download manager (M3) needs them.

## Proposal

Option 2, in this order: `serde_json` + `tracing` (M1), `rusqlite` (M2 history), then decide on
`cxx-qt` versus the current bridge with a measured comparison. Packaging stays Flatpak unless
a Deck test shows a problem.
