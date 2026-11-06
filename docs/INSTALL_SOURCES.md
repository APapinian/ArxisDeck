# Arxis Deck install sources

Decided on 2026-11-05: Arxis Deck installs from these sources. **Not** from AUR or pacman,
because SteamOS updates wipe the system partition.

Everything goes into the user's home folder, so no password is needed and SteamOS updates
keep it. Inside the Flatpak package, host tools (`flatpak`, `curl`, `tar`) are reached
through `flatpak-spawn --host`.

| Source | Status | Where it goes | How |
|---|---|---|---|
| **Flatpak (Flathub)** | ✅ done: search, install, update, uninstall, launch, icons | user Flatpak installation | `flatpak --user …` |
| **Emulators** | ✅ on Flathub (RetroArch, Dolphin, PCSX2, PPSSPP, RPCS3, Cemu, shadPS4) | user Flatpak installation | same as Flatpak |
| **Proton / compatibility tools** | ✅ Proton-GE: newest 3 releases, install (SHA-512 checked), uninstall | `~/.steam/root/compatibilitytools.d` | GitHub releases → `curl` + `tar` |
| **GitHub releases** (general) | 🔜 next: same download path as Proton-GE for any listed project | depends on the file type | GitHub releases API |
| **AppImage** | ✅ DuckStation, Vita3K, xemu (GitHub, SHA-256 checked); saved not runnable, "Make runnable" after confirmation; manifest-only uninstall | `~/Applications`, app-menu entry `arxis-deck-*.desktop` | GitHub releases (`src/appimage.rs`) |
| **Decky plugins** | 🔍 needs research | `~/homebrew/plugins` (owned by root) | through Decky Loader's own installer |

## Notes per source

### Proton / compatibility tools
- Releases come from `api.github.com/repos/GloriousEggroll/proton-ge-custom/releases`.
  Unauthenticated GitHub requests are limited to 60 per hour per address; Arxis Deck asks once per start.
- The `x86_64` `.tar.gz` is downloaded to a temporary folder, its `.sha512sum` is compared, then it is
  unpacked. The temporary folder is always removed.
- Steam has to be restarted to see a new tool; then it is chosen per game in
  **Properties → Compatibility**.
- Later: other tools from GitHub releases (e.g. Luxtorpeda, Boxtron) through the same path.

### GitHub releases (general)
- A short, reviewed list of projects (no free-form URLs from the controller).
- Only `https://github.com/…/releases/download/…` links are accepted.

### AppImage
- **Behaviour confirmed by the owner on 2026-11-06:** the file is saved to `~/Applications`
  without permission to run; only after the user confirms "Make runnable" is it marked runnable
  and added to the app menu, so Steam's "Add a Non-Steam Game" can find it.
- Needs FUSE on the device, or `APPIMAGE_EXTRACT_AND_RUN=1` as a fallback (added automatically to
  the app-menu entry when `fusermount` is missing).
- A `.desktop` entry in `~/.local/share/applications` lets Steam's **Add a Non-Steam Game** find it.
- Still to check on a Steam Deck: one real install → make runnable → add to Steam → start in Gaming Mode.

### Decky plugins
- Decky Loader itself is installed with root rights and keeps plugins in a root-owned folder,
  so Arxis Deck cannot write there directly without a password.
- To research: whether Decky Loader's local service can be asked to install a plugin from its
  store (`plugins.deckbrew.xyz`), so Arxis Deck only shows and requests plugins.
