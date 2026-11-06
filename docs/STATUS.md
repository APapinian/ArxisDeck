# Arxis Deck — status against ROADMAP.md

Updated 2026-11-06. The planning documents (PROJECT.md, ARCHITECTURE.md, ROADMAP.md) were only
found in the project folder after the first prototype slices were built, so work ran ahead of
the M0 review gate. Nothing below is "done" in the ROADMAP sense until it has been tested on a
Steam Deck (PROJECT.md §9).

| Milestone | State | What exists | Missing for acceptance |
|---|---|---|---|
| **M0** research | deliverables written; licence decided (ADR-0002) | `docs/research/arxis-analysis.md`, `license-review.md`, `steamos-compatibility.md`, `docs/architecture-decisions/ADR-0001-technology-stack.md`; Rust↔QML bridge proven | — |
| **M1** shell | prototype | controller/keyboard/touch navigation, 1280×800 scaling, settings in `~/.config/arxis-deck/config.toml`, structured logging (`tracing`, redacted), self-test, screenshot mode | SQLite + migrations, Desktop/Gaming presentation profiles, diagnostics/about page, `cargo fmt`/`clippy` gates, Deck test |
| **M2** Flatpak store | prototype, real installs verified on desktop | user-scope search/install/update/uninstall/launch, icons, progress, cancel, confirmation before uninstall | permissions and download size before install, operation history, offline/disk-full handling, Deck test |
| **M3** AppImage + GitHub | started | GitHub releases path for Proton-GE: HTTPS-only GitHub links, SHA-512 check (labelled "same release", not publisher proof), archive path checks, no overwrite, fixture test | AppImage provider, ownership manifest, signatures, user-added repositories, rate-limit handling |
| **M4** gaming tools | started | 20 curated Flathub tools/launchers/emulators (IDs verified 2026-11-05), Proton-GE | support status per entry, explanatory pages, Deck tests |
| **M5** Distrobox/AUR | not started (optional) | — | — |
| **M6** Gaming Mode polish | partly | motion, themes, own on-screen keyboard, reduced-motion not yet | Steam virtual keyboard, suspend/resume, accessibility options |
| **M7** packaging | prototype | Flatpak bundle on KDE runtime 6.11, build script, install guide | signing, CI, self-update route, compatibility matrix |

## Changed to follow the documents (2026-11-06)

- **Buttons:** X opens search, Y is the secondary action (ARCHITECTURE.md §3.3). Before: the reverse.
- **Uninstall:** a confirmation dialog shows name, source and scope, and focuses Cancel first
  (PROJECT.md §6). Before: pressing X twice.
- **Verification labels:** Proton-GE shows "SHA-512 (same release)" and explains it does not
  prove the publisher (PROJECT.md §6). Before: "SHA-512 checked".
- **Compatibility tools:** an existing folder is never overwritten; archives with paths outside
  the tool's folder or with `..` are refused (PROJECT.md §5.2, §6).
- **Settings file:** `config.toml` (ARCHITECTURE.md §8). An older `settings.conf` is read once.

## Known differences still open

- ADR-0001 step 1 is in: `serde`/`serde_json` and `tracing`, vendored, licences reviewed.
  Logs: `~/.local/share/arxis-deck/logs/arxis-deck.log`. Step 2 is in: SQLite operation history.
- Item-by-item gaps per milestone and the work order: `docs/Milestone.md`.
- All quality gates: `sh scripts/check.sh`.
- Own on-screen keyboard instead of the Steam virtual keyboard.
- Installing now asks first: the confirmation shows source, scope, version, sizes, licence and
  permissions (gap 3, `docs/Milestone.md`).
- No favourites yet. Operation history: ≡ in Downloads. About & diagnostics with a redacted
  report: Settings → last row (gap 4).
