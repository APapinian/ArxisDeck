# Putting Arxis Deck on a Steam Deck

Arxis Deck is packaged as a Flatpak, like most apps on SteamOS. Qt comes from the KDE
runtime on Flathub, so the package itself is small, and SteamOS updates don't remove it.

App ID: `io.github.fatih_bucaklioglu.ArxisDeck` (can still be changed before a public release).

## 1. Build the package (on a Linux PC, or on the Deck in Desktop Mode)

```sh
sh packaging/build-flatpak.sh
```

The first run downloads the build tools for your user from Flathub (about 1.5 GB). The
result is one file: `arxis-deck.flatpak`.

## 2. Install it on the Deck

1. Switch the Deck to **Desktop Mode** (Steam button → Power → Switch to Desktop).
2. Copy `arxis-deck.flatpak` to the Deck (USB stick, a network share, or download it into
   **Downloads**). Arxis Deck is not on Flathub yet, so it is installed from this file.
3. Open **Konsole** and run (first line only if the file is in Downloads):

   ```sh
   cd ~/Downloads
   flatpak install --user ./arxis-deck.flatpak
   ```

   Answer **y** when it asks to add Flathub and install the KDE runtime (about 400 MB the first
   time; the Deck may already have it). No password is needed.
4. To update later, run the same command with a newer file and `--reinstall`.

## 3. Add it to Steam so it opens in Gaming Mode

1. Still in Desktop Mode, open **Steam** → **Games** → **Add a Non-Steam Game to My Library…**
2. Tick **Arxis Deck** in the list and press **Add Selected Programs**.
3. Switch back to **Gaming Mode**. Arxis Deck is in **Library → Non-Steam**.
4. If the controller does nothing in Arxis Deck: open its page, press the controller icon
   and choose the **Gamepad** layout template.

## What the package is allowed to do

| Permission | Why |
|---|---|
| Talk to Flatpak on the Deck (`org.freedesktop.Flatpak`) | Install, remove, search and start apps, always for your user only |
| All devices | Read the controller and use the GPU |
| Read Flatpak's app data (read-only) | Show real app icons |
| Wayland / X11 windows | Draw the store (Gaming Mode uses X11 through gamescope) |

It has no internet access of its own: downloads are done by Flatpak itself.

## Already checked on a Linux PC (2026-11-05)

- The package builds offline and is 358 KB; it runs on the KDE runtime's Qt 6.11.
- Inside the sandbox all eight self-test steps pass.
- Installed for the user, it reaches the PC's Flatpak (lists apps, searches Flathub) and shows app icons.
- The runtime includes the `script` tool, so install progress bars work from inside the package.

## Still to check on a real Deck

- Full screen in Gaming Mode and controller input with Steam's default layout.
- Press X and Y once and read "Last input" at the bottom: X should say *secondary*, Y *search*.
  If they are the other way round, set Settings → **X and Y buttons** → **Swapped**.
- The clock: on the build PC the sandbox showed UTC because that PC's timezone file isn't
  visible inside the sandbox; SteamOS normally passes the timezone through.

## Open decisions

- **Licence:** decided 2026-11-06: GPL-3.0-or-later, © Fatih Bucaklioglu (ADR-0002). The package
  installs `LICENSE` and `THIRD-PARTY-LICENSES.md` into `/app/share/licenses/`.
- **Deck test:** the checklist is in `docs/DECK_TEST.md` (to do later).
