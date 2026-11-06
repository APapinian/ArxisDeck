# Arxis analysis (M0, task 1, 3, 4)

Source inspected: `references/arxis/` (read-only), git remote `https://github.com/APapinian/Arxis.git`,
4 commits on 2026-03-18 (`a14eee1` … `7437cd7`). Inspected 2026-11-05.

## Structure

| Path | Lines | Role |
|---|---|---|
| `main.py` | 37 | PyQt6 entry point, creates `ui.main_window.ArchPackageStore` |
| `ui/main_window.py` | 4863 | Whole UI: sidebar, pages, dialogs, context menus |
| `ui/styles.py`, `ui/theme.py` | 336, 303 | Theme table (`THEMES`), `build_theme()` |
| `ui/widgets.py`, `ui/icons.py` | 356, 49 | Custom widgets, glyph icons |
| `backend/managers.py` | 1841 | Providers: `PacmanManager`, `AURManager`, `FlatpakManager`, `AppImageManager`, `WineManager` (defined twice, lines 1148 and 1175), `PackageManagerHub`, `GitHubReleaseManager`, `SnapshotManager` |
| `backend/package_manager.py` | 324 | Older, parallel provider code (`PackageManager`), apparently unused |
| `backend/system_monitor.py`, `backend/workers.py` | 94, 42 | CPU/RAM/network threads, search worker |
| `install.sh`, `run.sh`, `run.fish`, `PKGBUILD` | — | venv setup, launchers, Arch package (needs `arxis.spec`, not in the repo) |

## Command execution paths

| Provider | Commands | Privilege |
|---|---|---|
| Pacman | `pacman -S/-Rs/-Syu/-Sc …` | `sudo -S` with the password piped from memory (`BaseManager._sudo_password`) |
| AUR | `yay` / `paru` | helper asks for sudo |
| Flatpak | `flatpak install -y flathub <id>` (no `--user`), Flathub API for details | system scope |
| AppImage | `appimage.github.io/feed.json`, GitHub API, download to `~/.local/share/AppImages/` | user |
| GitHub releases | download asset, then `dpkg -i`, or `make && sudo make install`, or meson/ninja install | **sudo**, runs build scripts |
| Snapshot | lists packages from all providers to JSON, diff, re-install | depends on provider |

## Reuse decision

| Part | Decision | Why |
|---|---|---|
| Python/PyQt6 code | **not reused** | Different language and toolkit (Rust + QML); mouse-first widgets |
| Pacman / AUR / sudo paths | **must not be carried over** | Violate PROJECT.md §6 (no host pacman/AUR/sudo) |
| `GitHubReleaseManager._auto_install` | **must not be carried over** | Runs project build scripts with sudo |
| Flatpak provider | **redesigned** | Arxis Deck uses `--user` scope, progress, cancel |
| AppImage feed idea, GitHub release lookup | **idea reused, code rewritten** | Planned for M3 with verification and a managed-file manifest |
| Feature list (Discover, Installed, Categories, Queue, Settings, themes) | **idea reused** | Matches the Arxis Deck tabs |
| Theme colours | **8 background/accent pairs reused as data** from the README theme table | See license review |
| Snapshot, history, favourites | **later** | PROJECT.md §5.3 asks for history and favourites |

Other findings: interface strings are hard-coded Turkish; `WineManager` is defined twice (the
second shadows the first); `package_manager.py` duplicates `managers.py`; README file list omits
`theme.py`, `widgets.py`, `icons.py`, `workers.py`, `package_manager.py`.
