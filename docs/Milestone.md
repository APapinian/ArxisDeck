# Milestone gaps (from ROADMAP.md) — docs/Milestone.md

Checked 2026-11-06 against every task, check and acceptance criterion in ROADMAP.md M0–M7.
Legend: ✅ done · 🟡 partly · ❌ missing · 🔒 needs a real Steam Deck.
A milestone only counts as complete when all its acceptance criteria pass (ROADMAP.md rule).

## M0 — Research and feasibility

| Item | State | Evidence / gap |
|---|---|---|
| Inspect Arxis, record structure | ✅ | `docs/research/arxis-analysis.md` |
| Repository identity, licences, notices | ✅ | `docs/research/license-review.md`; Arxis Deck is GPL-3.0-or-later, © Fatih Bucaklioglu (ADR-0002); no Arxis code is used, so the Arxis licence/copyright question only matters before taking any |
| Map providers and command paths | ✅ | arxis-analysis.md |
| Reusable / redesign / must-not-carry | ✅ | arxis-analysis.md |
| SteamOS constraints | ✅ | `docs/research/steamos-compatibility.md` |
| Rust/Qt/QML prototype builds | ✅ | builds native and as Flatpak |
| QML receives a typed event from Rust | ✅ | `backendEvent(kind, a, b)` |
| ADR-0001 | ✅ | accepted 2026-11-06 |
| Updated ARCHITECTURE.md | ❌ | not edited (owner's file); differences listed in ADR-0001 and STATUS.md |
| **Acceptance:** licence and reuse decision documented | ✅ | ADR-0002 + license-review.md |

## M1 — Application skeleton and controller-first shell

| Item | State | Evidence / gap |
|---|---|---|
| Module boundaries | 🟡 | `input`, `store`, `github`, `settings`, `logging`, `qt` modules; not yet the `providers/`, `core/` tree of ARCHITECTURE.md §4 |
| QML shell | ✅ | `qml/Main.qml` (state + navigation) with `pages/`, `components/`, `themes/`, bundled into the program as a Qt resource |
| Bootstrap, structured logging, configuration | ✅ | `tracing` with redaction; `config.toml` |
| SQLite + migrations | ✅ | `src/history.rs`, `PRAGMA user_version` migrations |
| Navigation, Gaming/Desktop presentation profiles | 🟡 | navigation ✅; no separate Desktop profile (one scaled layout) |
| Controller focus, hints, keyboard/mouse | ✅ | self-test steps move/act/search/menu |
| Diagnostics/about page | ✅ | Settings → About & diagnostics (`src/diagnostics.rs`) |
| Tests: configuration and migrations | ✅ | `settings::tests`, `history::tests` |
| Clean and existing profile start | 🟡 | settings migration from old `settings.conf` tested; no end-to-end test |
| No UI freeze during background work | ✅ | all jobs on threads; UI ticks during installs |
| 1280×800 and resizable window | 🟡 | 1280×800 ✅; resize only scales |
| 🔒 Runs on Steam Deck | ❌ | needs hardware |
| Quality gates fmt/clippy/test | ✅ | `scripts/check.sh` |

## M2 — Flatpak provider and first usable store

| Item | State | Evidence / gap |
|---|---|---|
| Detect Flatpak, installed apps | ✅ | `--flatpak-probe` |
| Remotes and catalogue metadata | 🟡 | Flathub user remote; curated list + `flatpak search`; no remote management UI |
| Search, categories, detail pages | ✅ | |
| User-scope install | ✅ | 3 real round trips |
| Progress, errors, cancellation | ✅ | pseudo-terminal progress; cancel |
| Operation history | ✅ | SQLite, ≡ in Downloads |
| Update checks, user-confirmed updates | ✅ | X / ≡ in Installed |
| Uninstall with user-data handling | 🟡 | confirmation says data in `~/.var/app` stays; no option to delete it |
| See source, scope, version and permissions before install | ✅ | `src/details.rs` (`flatpak remote-info` + `--show-metadata`, plain-language permissions with attention levels, tests with recorded output); details page shows version, download, permission count; install confirmation shows source, scope, version, sizes, licence, runtime and permissions; Install waits for the details |
| Tests: offline, missing remote, disk full, interrupted | 🟡 | offline (self-test step + unit test), disk full (script test), interrupted (history test) ✅; missing remote ❌ |
| Installed state from Flatpak, not the cache | ✅ | always `flatpak list` |
| 🔒 Install/update/uninstall from Gaming Mode | ❌ | needs hardware |

## M3 — AppImage and GitHub Releases

| Item | State | Evidence / gap |
|---|---|---|
| GitHub Releases provider | 🟡 | Proton-GE only (`src/github.rs`) |
| Architecture/format matching | ✅ | explainable rules + tests |
| Download manager (pause/resume only if safe) | 🟡 | cancel only |
| Staging, checksum, safe finalisation | 🟡 | temp folder + SHA-512 + path checks; no signatures |
| AppImage registration/removal + manifest | ✅ | `src/appimage.rs`: staging → size + SHA-256 (GitHub digest) + ELF checks → `~/Applications` without exec bit; "Make runnable" after confirmation (chmod 755 + `arxis-deck-*.desktop`); manifest `appimages.json`; uninstall only manifest files |
| Trust/verification labels | 🟡 | Proton details; not on cards |
| User-managed GitHub repositories | ❌ | |
| Tests: bad checksum, traversal, no auto-execution | 🟡 | traversal, overwrite, checksum mismatch, free space ✅; rate-limit message ✅ (unit test); redirects ❌ |

## M4 — Gaming tools

| Item | State | Evidence / gap |
|---|---|---|
| Curated catalogue with support status | 🟡 | 20 verified IDs; no per-entry support status |
| Map to installation source | ✅ | Flathub / GitHub |
| Detect installed, avoid duplicates | ✅ | badges, no-overwrite |
| Proton tool paths | ✅ | compatibilitytools.d |
| Confirmation + preflight for compat tools | 🟡 | uninstall confirmed; install has no preflight (free space) |
| Explanatory pages | 🟡 | short summaries |

## M5 — Distrobox / AUR (optional)

❌ Not started — by design, disabled by default until M5 tests pass.

## M6 — Gaming Mode polish

| Item | State | Evidence / gap |
|---|---|---|
| Home, featured, categories | 🟡 | no "recently updated" |
| Focus and transitions | ✅ | |
| Steam virtual keyboard | 🟡 | Settings → Search keyboard: Steam; View (⧉) in search asks Steam (`steam://open/keyboard`) in Gaming Mode; Arxis keyboard stays as fallback; 🔒 not yet tried on a Deck |
| Download queue and overlays | ✅ | |
| Offline / empty / loading / error states | ✅ | offline label + subtitle, queue waits and resumes, "Searching Flathub…", actionable failure reasons |
| Accessibility: text size, reduced motion | ✅ | Settings → Text size (100/115/130 %) and Motion (Full/Reduced); self-test step "access" |
| 🔒 Suspend/resume, display, input reconnect | 🟡 | controller hot-plug in code; untested |

## M7 — Packaging and release

| Item | State | Evidence / gap |
|---|---|---|
| Packaging format | ✅ | Flatpak on KDE 6.11 |
| Reproducible build | 🟡 | offline vendored build script; no CI |
| Checksums and signing | ❌ | |
| Self-update route | ❌ | |
| Diagnostics export with redaction | ✅ | Y on the About page saves `arxis-deck-diagnostics-<time>.txt` (sections + last 200 log lines, home shown as `~`) |
| Uninstall and data-retention controls | ❌ | |
| Compatibility matrix | ❌ | |
| Third-party licence texts in releases | ✅ | `THIRD-PARTY-LICENSES.md` (31 crates, generated from `vendor/`, gate 6 of `scripts/check.sh`), installed with `LICENSE` in the Flatpak |

## Priority order

1. **Quality gates** — `scripts/check.sh` (fmt, clippy, tests, self-test) and a clean tree.
2. **SQLite + migrations + operation history** (M1, M2, PROJECT §5.3).
3. **Version, sizes and permissions before install** (M2 acceptance, PROJECT §5.1).
4. **Diagnostics/about page** (M1 task 7, PROJECT §5.5).
5. Offline / loading / error states; rate-limit and checksum-mismatch tests (M2, M3).
6. AppImage provider with ownership manifest (M3).
7. Split `Main.qml` into pages/components (ARCHITECTURE.md §3.2).
8. Steam virtual keyboard, reduced motion, text size (M6) — partly needs a Deck.
9. Owner decisions: licence, copyright; Deck test session (🔒 items).

## Progress log

| Date | Gap | Result |
|---|---|---|
| 2026-11-06 | 1 Quality gates | ✅ `scripts/check.sh`: `cargo fmt --check` (with `rustfmt.toml`, 120 columns), `cargo clippy -D warnings`, `cargo test` (30 tests), QML loads/draws, 10-step controller self-test — all pass |
| 2026-11-06 | 2 SQLite + migrations + history | ✅ `src/history.rs`: `rusqlite` (bundled SQLite, vendored, licences reviewed), numbered migrations (`PRAGMA user_version`), operations recorded with time/source/result, unfinished ones marked "interrupted" at the next start; ≡ in Downloads opens the activity history; tests for migrations, reopen and interruption; self-test step "history" |
| 2026-11-06 | 3 Version, sizes, permissions before install | ✅ chosen by the owner; `src/details.rs` + install confirmation dialog (Flathub and Proton-GE), details prefetched for the focused card, 4 parser tests on real recorded output, self-test checks the dialog lists permissions. Live check: Heroic v2.22.4, 325.2 MB download, 22 permissions |
| 2026-11-06 | 4 Diagnostics/about page | ✅ Settings → About & diagnostics: version, Qt (from the bridge), package, licence status, OS (host os-release inside the sandbox), Gaming Mode, window, language, store mode/scope, controllers (with PlayStation layout note), settings/history/log files; Y exports a redacted report to `~/.local/share/arxis-deck/`; 2 unit tests, self-test step "about" (11 steps) |
| 2026-11-06 | 5 Offline / loading / error states | ✅ Rust checks Flathub (`curl -I`, every 30 s offline / 5 min online) and reports changes; top bar "⚠ Offline" + red subtitle; queued installs wait ("Waiting for a connection") and start by themselves when back online, then lists refresh; `friendly_error` turns raw errors into advice (no internet, no space, damaged download, GitHub rate limit); failed rows show the reason; Flathub search shows searching / results / failure; Proton free-space preflight (3× download); tests: 3 new unit tests, checksum-mismatch and low-space script cases, self-test step "offline" (12 steps) |
| 2026-11-06 | 6 AppImage provider with ownership manifest | ✅ (behaviour confirmed by the owner 2026-11-06: saved not runnable, "Make runnable" only after confirmation) curated DuckStation, Vita3K, xemu from GitHub `releases/latest` (asset rules tested on real names; SHA-256 from GitHub's digest, labelled integrity-only); download to `~/.local/share/arxis-deck/staging`, size/checksum/ELF checks, moved to `~/Applications` not executable; Installed shows "Saved · not runnable", A asks "Make runnable" (explains it runs outside any sandbox) then chmod + app-menu entry so Steam's "Add a Non-Steam Game" finds it; manifest lists every created file, uninstall refuses anything else; 5 new tests (script fixtures incl. wrong checksum/size, non-ELF, no overwrite, refusal outside managed paths) + self-test step "appimage" (13 steps). Not yet: a real AppImage install on a device, user-added repositories, signatures |
| 2026-11-06 | 7 Split `Main.qml` into pages/components | ✅ no behaviour change: `qml/` is bundled with Qt's `rcc` at build time and loaded from `qrc:/qml/Main.qml`; Main.qml went from 3,263 to about 1,350 lines (state, navigation, the card grid); 9 pages in `qml/pages/` (Library, Category, Downloads, Settings, Application details, Search, Menu, About, History) and 10 parts in `qml/components/` (TopBar, SectionHeader, ControllerHints, Toast, ConfirmDialog, Splash, AmbientBackground, Art, Orb, ButtonGlyph), colours in `qml/themes/Theme.qml`; every `--open` screen drawn with no QML warnings, all gates pass (13/13 self-test). Folder is `qml/` rather than ARCHITECTURE's `ui/` |
| 2026-11-06 | 8 Steam virtual keyboard, reduced motion, text size | ✅ in code, 🔒 Steam keyboard still to try on a Deck. Settings gained Text size (Normal/Large/Largest = 100/115/130 %, every font goes through one scale; long text now ends with "…" instead of running into buttons), Motion (Reduced: one-off animations take 0 ms, background drift and pulsing rings stop) and Search keyboard (Arxis/Steam); the settings list scrolls. In Gaming Mode, opening search with "Steam" chosen, or View (⧉) in search, asks the running Steam for its on-screen keyboard (`steam steam://open/keyboard`, through `flatpak-spawn --host` in the package); its keys arrive as normal key presses the search already accepts. The automated runs never call Steam. New `--setting key=value` for screenshots; self-test step "access" (14 steps) |
| 2026-11-06 | 9 Owner decisions and Deck test | ✅ licence: the owner asked me to decide → GPL-3.0-or-later, © 2026 Fatih Bucaklioglu (ADR-0002, changeable); full `LICENSE`, `Cargo.toml` licence/authors, SPDX lines in every source file, Flatpak metainfo `project_license` + `developer`, licence texts installed in the package, About shows the licence; `THIRD-PARTY-LICENSES.md` + check gate 6. 🔒 Deck test: postponed by the owner ("test later"); 19-point checklist in `docs/DECK_TEST.md` |
| 2026-11-06 | 9 check + Deck build | ✅ gap 9 re-checked (LICENSE = GPL-3.0 text, licence in Cargo.toml/metainfo/About, SPDX lines, third-party texts in the package). Version 0.2.0 built as `arxis-deck.flatpak` (1.3 MB, needs `org.kde.Platform//6.11`, fetched from Flathub by the bundle's runtime-repo); metainfo passes `appstreamcli validate` (homepage added), menu entry passes `desktop-file-validate`; installed copy reports 0.2.0 / GPL-3.0-or-later and passes 14/14 self-test steps inside the sandbox. App details now fall back to the system Flathub remote (SteamOS has no user remote before the first install). Install steps for the Deck in `packaging/README.md` |
| next | Deck test session (`docs/DECK_TEST.md`); then general GitHub releases, Decky plugins research, user-added repositories | |
