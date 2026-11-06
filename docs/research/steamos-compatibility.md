# SteamOS compatibility (M0, task 5)

Status of each constraint: **verified** (tested here), **documented** (from published sources),
or **to test on a Deck**. Development machine: NixOS, Qt 6.11.2, Flatpak 1.16.6.

## Host integrity

| Constraint | Arxis Deck behaviour | Status |
|---|---|---|
| System partition is read-only; pacman/AUR changes are wiped by updates | never calls pacman, AUR helpers or sudo | design rule, enforced in code review |
| No `steamos-readonly disable` | not present anywhere | verified (`grep`) |
| Flatpak apps for the user only | always `flatpak --user`; `remote-add --user --if-not-exists flathub` before the first install | verified: 3 real install/uninstall round trips of `org.gnome.TextEditor` |
| Steam compatibility tools | `~/.steam/root/compatibilitytools.d` (fallback `~/.local/share/Steam/…`), no root | documented; to test on a Deck |
| Settings | `~/.config/arxis-deck/config.toml` (Flatpak: `~/.var/app/<id>/config/…`) | verified |

## Running in Gaming Mode

| Question | Finding | Status |
|---|---|---|
| Packaging | Flatpak on `org.kde.Platform//6.11` (Qt 6.11), 358 KB bundle; host tools through `flatpak-spawn --host` | verified on desktop (sandbox self-test passes, real Flathub list/search/icons) |
| Full screen 1280×800 | full screen when `GAMESCOPE_WAYLAND_DISPLAY` or `SteamDeck=1` is set; layout scales uniformly | to test on a Deck |
| Controller | evdev from `/dev/input` (`--device=all`), Steam Input virtual Xbox-style pad: X → `BTN_X` (0x133), Y → `BTN_Y` (0x134) | documented (SDL #6068, xpad); Settings → "X and Y buttons" as fallback; to test on a Deck |
| Steam virtual keyboard | Arxis Deck has its own on-screen keyboard; Steam's keyboard is not triggered yet | **gap** vs PROJECT.md §4/§7; to test on a Deck |
| Install progress | flatpak only shows progress on a terminal: run through util-linux `script` without `--noninteractive` | verified (28 progress updates in a real install) |
| Host language inside the package | `flatpak-spawn --host` does not pass the sandbox's environment, so host `flatpak` answered in the user's language (Turkish "Sürüm" instead of "Version"); every host command now gets `--env=LC_ALL=C` (launching apps keeps the user's language) | verified 2026-11-06: sandboxed probe reads Heroic's version and sizes |
| Timezone inside the sandbox | showed UTC on the NixOS build PC | to check on a Deck |
| Adding to Steam | user-guided "Add a Non-Steam Game" (`packaging/README.md`); no VDF editing | per ARCHITECTURE.md §10 |

## Open items for the first Deck session

1. Gaming Mode full screen and focus.
2. Press X and Y once; "Last input" must say `search` and `secondary`.
3. Steam keyboard behaviour on a search field.
4. Sleep/resume while a download runs.
5. One Flatpak install/uninstall and one Proton-GE install/uninstall.
