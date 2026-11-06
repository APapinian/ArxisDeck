# Arxis Deck: interaction sheet (M1 test window)

Every number that shapes how the app looks and responds, with where it came from.
"estimated" means it's a starting value to be checked on a real Steam Deck.
The same values are in `mechanics.json`; the code uses them as written here.

## Display

| Value | Setting | Source |
|---|---|---|
| Design width | 1280 px | user request (Steam Deck screen) |
| Design height | 800 px | user request |
| Other screen sizes | uniform scale to fit, centred | estimated |
| Full screen | automatic when `GAMESCOPE_WAYLAND_DISPLAY` is set or `SteamDeck=1`; `--fullscreen`/`--windowed` override | estimated |
| Update tick | 16 ms | estimated |
| Screenshot | after 120 drawn frames | Craft Anything requirement |

## Controller buttons → actions

| Button | Action | Source |
|---|---|---|
| D-pad / left stick | move focus | user request (full controller navigation) |
| A | open / confirm | estimated (Steam convention) |
| B | back / close (on the first section: hint to leave with the Steam button) | estimated (Steam convention) |
| X | search: opens the on-screen keyboard (in Installed: update the selected app) | ARCHITECTURE.md §3.3 |
| X / Y codes | Xbox-style pads and Steam Input's virtual pad (Gaming Mode) send printed labels: X → BTN_X (0x133), Y → BTN_Y (0x134); PlayStation pads without Steam Input follow the kernel's position rule and are detected by name ("Sony", "DualSense", "DualShock", "Wireless Controller"…) and mapped by position | SDL issue #6068, xow issue #2, Arch FS#77845, Firefox bug 1903371, kernel gamepad.rst |
| Y | secondary action: add to / take out of Downloads; in Installed: uninstall (with confirmation) | ARCHITECTURE.md §3.3 |
| L1 / R1 | previous / next section | estimated (Steam convention) |
| L2 / R2 | page up / down (3 rows) | estimated |
| Start (☰) | options menu (replaces right-click) | estimated |
| Select (⧉) | Downloads section | estimated |
| Steam / ••• | not used: reserved by Steam | estimated |

Keyboard fallback: arrows, Enter/Space, Esc/Backspace, Q/E, PageUp/PageDown, X, Y or /, M, Tab.
Touch: tapping a card or tab does the same as focusing it and pressing A.

## Search (on-screen keyboard)

| Value | Setting | Source |
|---|---|---|
| Layout | 5 rows, 10 key units each: digits · qwertyuiop · asdfghjkl- · zxcvbnm.'⌫ · Space(5) Clear(2) Done(3) | estimated |
| Starting key | "q" | estimated |
| Up/down between rows | goes to the key under the centre of the current key | estimated |
| A | type the focused key | estimated |
| Y | delete one character | estimated |
| X | space | estimated |
| Start or B | close the keyboard, keep the text | estimated |
| Longest search text | 24 characters | estimated |
| What it matches | item name or category, anywhere in the text, ignoring case | estimated |
| Filtering | live, while typing | estimated |
| B in search results | clears the search | estimated |
| Changing section | clears the search | estimated |
| Physical keyboard while searching | letters, digits, space, - . ' type; Backspace deletes; Enter/Esc close | estimated |

## Options menu (Start)

| Value | Setting | Source |
|---|---|---|
| Entries | Open details · Search · Add to queue · About this test build · Close | estimated |
| Panel | 400 px wide, slides in from the right | estimated |
| Up/Down | move; stops at the ends | estimated |
| A | choose | estimated |
| Start or B | close | estimated |

## Store backend

| Value | Setting | Source |
|---|---|---|
| Real source | Flathub, installed with `flatpak install --user --noninteractive -y flathub <id>` (no password; kept by SteamOS updates) | person's choice (2026-11-05) |
| Flathub remote | added for the user with `--if-not-exists` before the first install | estimated |
| Uninstall | `flatpak uninstall --user --noninteractive -y <id>`, only after a confirmation dialog that shows name, source and scope; Cancel is focused first | PROJECT.md §6 |
| Installed list | `flatpak list --user --app`, refreshed after every install/uninstall | estimated |
| Search | Y also runs `flatpak search` (500 ms after typing stops, 2+ letters); Flathub results join the catalogue as category "Flathub" (max 40) | estimated |
| Starter list | 20 Steam Deck apps (compatibility tools, launchers, tools, emulators, utilities); every ID found with `flatpak search` on 2026-11-05 | flatpak search |
| Progress | flatpak runs on a pseudo-terminal (util-linux `script`, without `--noninteractive`, which hides progress); "Installing k/n" steps and "NN%" give the bar, never moving backwards within a step, capped at 99 % until done; with no percentage yet the bar shows a moving stripe | real install of org.gnome.TextEditor on 2026-11-05 (28 progress updates; piped output gave none) |
| Real round trip | install → listed → uninstall → gone, `--flatpak-roundtrip <id>` | passed 3 times on 2026-11-05 (163 s, 10.5 s, 13.6 s) |
| Icons | Flathub icon from flatpak's local appstream data: `~/.local/share/flatpak/appstream/flathub/<arch>/active/icons/128x128/<id>.png`, then `/var/lib/flatpak/...`; gradient art when missing | found locally on 2026-11-05 (3,363 icons) |
| Launch | A on an installed app (Installed, a finished download, or the details page) runs `flatpak run <id>` detached, in the user's own language | estimated |
| Update check | `flatpak remote-ls --user --updates --app` at start-up and after every install, update and uninstall (Arxis Deck itself is left out) | estimated |
| Updating | `flatpak update --user -y <id>` through Downloads with the same progress bar; in Installed Y updates the selected app, ≡ updates all | estimated |
| Update count | badge on the Installed tab; "Update available" on the app's row and card | estimated |
| Simulated updates | sample items with an even number have one until updated | estimated |
| Proton-GE | newest 3 stable releases from GitHub (`api.github.com/repos/GloriousEggroll/proton-ge-custom/releases`), shown as cards in Compatibility with a "GitHub" badge | GitHub API, 2026-11-05 |
| Proton-GE install | x86_64 `.tar.gz` → temporary folder → SHA-512 compared with the release's `.sha512sum` → unpacked into `~/.steam/root/compatibilitytools.d` (or `~/.local/share/Steam/…`); progress from curl's bar | estimated (not run on a device with Steam yet) |
| Proton-GE in Installed | listed as "Steam compatibility tool"; A explains Properties → Compatibility; Y + confirmation takes the folder away; an existing folder is never overwritten on install | estimated |
| Install confirmation | Y on a card (or Install on the details page) opens a dialog: source and scope, version, download and installed size, licence, runtime, permissions (high ones marked "⚠ attention"); Cancel focused first; Install waits until the details are loaded | PROJECT.md §6, ROADMAP.md M2 |
| Details source | `flatpak remote-info --user flathub <id>` and `--show-metadata`, fetched when the details page or dialog opens, and 600 ms after focus rests on a card (real store) | `flatpak remote-info` output recorded 2026-11-06 |
| Permission levels | high: all files, whole home, running commands outside the sandbox, session/system bus, SSH/GPG keys; medium: internet, sound, all devices, drives, named folders; low: windows, graphics card, app-specific folders | estimated |
| AppImages | DuckStation, Vita3K, xemu from GitHub `releases/latest`; file rules: `.AppImage`, not ARM/debug/SSE2, prefer x86_64/x64/amd64 | GitHub API, checked 2026-11-06 |
| AppImage install | staging `~/.local/share/arxis-deck/staging` → size, SHA-256 (GitHub digest), ELF check → `~/Applications/<file>` mode 0644; free space 1.1× size | PROJECT.md §5.2, ARCHITECTURE.md §5.2 |
| Make runnable | A on a saved AppImage in Installed → confirmation → `chmod 755` + `~/.local/share/applications/arxis-deck-<owner>.<repo>.desktop` (adds `APPIMAGE_EXTRACT_AND_RUN=1` when FUSE is missing) | PROJECT.md §5.2 |
| AppImage manifest | `~/.local/share/arxis-deck/appimages.json`; uninstall removes only the listed AppImage and .desktop file | ARCHITECTURE.md §5.2, §9 |
| Connection check | `curl -I` to Flathub's repo file (6 s timeout), every 30 s while offline, every 5 min while online; on coming back: Proton list and updates refresh | estimated |
| Offline behaviour | top bar "⚠ Offline", red subtitle in Discover/Downloads, queued installs show "Waiting for a connection" and start by themselves when online | PROJECT.md §5.3, §7 |
| Error messages | no internet → "connect, then press A to retry"; no space → "Not enough free space (need … have …)"; checksum mismatch → "download was damaged"; GitHub 403/429 → "hourly limit"; shown on the failed row | PROJECT.md §5.3 |
| Proton free-space check | needs 3× the download size free before downloading | estimated |
| Inside a Flatpak sandbox | commands go through `flatpak-spawn --host` | estimated |
| Simulated store | used by `--screenshot`, `--self-test`, `--simulated`, `--open`, or when Flatpak is missing; +5 % every 100 ms | estimated |
| Quitting during an install | the flatpak process keeps running and finishes on its own | estimated |

## Downloads

| Value | Setting | Source |
|---|---|---|
| X on a card | add it to Downloads; again removes it | estimated |
| Select (⧉) | jump to the Downloads section | estimated |
| Details page first button | Add to Downloads / Remove download | estimated |
| Options menu entry 3 | Add to / Remove from Downloads | estimated |
| Order | one item at a time, in the order added; paused items are skipped | estimated |
| Simulated speed | +5 % every 100 ms (≈ 2 s per item) | estimated |
| In Downloads: Up/Down | choose a row; L2/R2 jump to first/last | estimated |
| In Downloads: A | pause / resume (on a finished item: notice) | estimated |
| In Downloads: X | remove the row | estimated |
| Card badge | In Downloads · Downloading n % · Paused · ✓ Downloaded | estimated |
| Top bar | "↓ n" active downloads with the current item's progress | estimated (in-app progress because Gaming Mode shows no desktop notifications) |
| Kept between runs | no | estimated |

## Installed (second section)

| Value | Setting | Source |
|---|---|---|
| Contents | every finished (simulated) download, in the order added | estimated |
| Up/Down | choose a row; L2/R2 jump to first/last | estimated |
| A | launch the app (`flatpak run`) | estimated |
| Y | uninstall, after a confirmation dialog | PROJECT.md §6 |
| Entering the section | selects the first row | estimated |

## Categories (third section)

| Value | Setting | Source |
|---|---|---|
| Layout | 3 × 2 tiles of 376 × 258 px | estimated |
| Categories | Tool, Emulator, Compatibility, Utility, Launcher, Plugin (4 Discover items each) | estimated |
| D-pad | moves between tiles; stops at the edges | estimated |
| A | opens Discover showing only that category | estimated |
| B in a category view | returns to Categories | estimated |
| Changing section | clears the category filter | estimated |

## Settings (last section)

Up/Down picks a row, Left/Right changes its value (stops at the ends), A steps to the next
value (wrapping) or runs Reset, L2/R2 jump to the first/last row, B goes back to Discover.
Saved as TOML (`theme = "Arch Midnight"`) in `~/.config/arxis-deck/config.toml` (or `$XDG_CONFIG_HOME`; ARCHITECTURE.md §8),
only in normal runs; screenshots use the defaults unless `--setting key=value` is given.
The list is taller than the page and scrolls to keep the focused row in view (two rows above it).

| Setting | Options (default first) | Source |
|---|---|---|
| Theme | Arch Midnight, Ocean Depth, Tokyo Night, Catppuccin Mocha, Nord Arctic, Gruvbox Warm, Dracula Pro, Everforest | background and accent from the Arxis README theme table; other shades estimated |
| Hold-to-repeat speed | Normal (400 ms, then every 110 ms), Slow (500 / 160), Fast (300 / 70) | estimated |
| Clock | 24-hour, 12-hour | estimated |
| Button hints | Show, Hide | estimated |
| Background glow | On, Off | estimated |
| X and Y buttons | Automatic, Swapped (exchanges the two for any controller) | estimated |
| Text size | Normal (100 %), Large (115 %), Largest (130 %): every font in the app; long text ends with "…" | PROJECT.md §7; percentages estimated |
| Motion | Full, Reduced: slides, zooms and fades finish at once; background drift and pulsing rings stop; the search caret still blinks and unknown-length download bars still move | PROJECT.md §7 |
| Search keyboard | Arxis, Steam: with Steam, opening search in Gaming Mode also asks Steam for its on-screen keyboard (`steam steam://open/keyboard`); in search, View (⧉) asks for it any time; its keys arrive as normal key presses. Only in Gaming Mode (gamescope or `SteamDeck=1`); not yet tried on a Deck | PROJECT.md §7, ROADMAP.md M6 |
| Reset to defaults | — | estimated |
| About & diagnostics | A opens: version, Qt, package, OS, Gaming Mode, window, language, store, controllers, files; ▲▼ scroll 80 px; Y saves a report (+ last 200 log lines, home shown as `~`) to `~/.local/share/arxis-deck/`; B closes | ROADMAP.md M1 task 7, PROJECT.md §5.5 |
| Saving delay | checked every 15 ticks (~0.25 s) | estimated |

Theme shades: panels, cards and borders are the theme background tinted toward slate #8fa6d6
by 5.5 % (panel), 7.5 % (card), 11 % (focused card), 12 % (raised), 14 % (border), 20 % (glyph).

## Navigation feel

| Value | Setting | Source |
|---|---|---|
| Hold-to-repeat delay | 400 ms | estimated |
| Hold-to-repeat interval | 110 ms | estimated |
| Stick / trigger press threshold | 0.60 of full travel | estimated |
| Stick / trigger release threshold | 0.35 of full travel | estimated |
| Controller rescan (hot-plug) | every 2000 ms | estimated |
| Grid columns | 4 | estimated |
| Page jump (L2/R2) | 3 rows | estimated |
| Wrap at row end | no | estimated (Steam behaviour) |
| Wrap at last section | no | estimated |
| Focus remembered per section | yes | estimated |

## Layout (at 1280×800)

| Value | Setting | Source |
|---|---|---|
| Top bar height | 72 px | estimated |
| Button hint bar height | 48 px | estimated |
| Card size | 276 × 162 px | estimated |
| Grid cell | 300 × 186 px | estimated |
| Focus outline | 3 px accent | estimated |
| Focus zoom | 1.04× | estimated |
| Section title text | 30 px | estimated |
| Body text | 16–18 px | estimated (readable on a 7" screen) |
| Smallest focus target | 48 px | estimated |
| Background colour | #070b14 | Arxis README theme table, "Arch Midnight" |
| Accent colour | #f97316 | Arxis README theme table, "Arch Midnight" |

## Packaging (Steam Deck)

| Value | Setting | Source |
|---|---|---|
| Format | Flatpak bundle `arxis-deck.flatpak`, app ID io.github.APapinian.ArxisDeck | estimated (ID can still change) |
| Runtime | org.kde.Platform 6.11 (Qt 6.11), built with org.kde.Sdk 6.11 + rust-stable 25.08 | `flatpak info` of the installed KDE runtime, 2026-11-05 |
| Sandbox permissions | ipc, wayland, fallback-x11, all devices (controller + GPU), talk to org.freedesktop.Flatpak, read-only Flathub appstream folders | estimated |
| Getting it into Gaming Mode | install the bundle in Desktop Mode, then Steam → Add a Non-Steam Game | estimated |
