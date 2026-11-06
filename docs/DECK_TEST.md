# Steam Deck test session (gap 9, 🔒)

The owner chose to test later. This is the list for that session; each line closes a 🔒 item
in `docs/Milestone.md`. Write the result (✅/❌ and a note) next to each line.

## Before you start

1. Copy `arxis-deck.flatpak` to the Deck (Desktop Mode) and install it for your user:
   `flatpak install --user ./arxis-deck.flatpak`
2. Steam → **Add a Non-Steam Game** → Arxis Deck. Return to Gaming Mode and start it from the library.
3. Settings → About & diagnostics → **Y** saves a report. Keep it with your notes.

## Screen and controller

| # | Check | Expected | Result |
|---|---|---|---|
| 1 | Start from Gaming Mode | full screen, 1280 × 800, no black bars; About shows "Gaming Mode: yes" | |
| 2 | D-pad, left stick, A, B, L1/R1, L2/R2, Start (≡), View (⧉) | each one moves or acts as the hints say | |
| 3 | Press X, then Y; read "Last input" | X = *search*, Y = *secondary*; if swapped, set Settings → X and Y buttons → Swapped | |
| 4 | Hold the D-pad down | focus repeats smoothly and the grid scrolls | |
| 5 | Turn the controller off and on (or plug one in) | the status at the bottom updates | |
| 6 | Touch a card, a tab and a setting | it works like pressing A on it | |

## Search and the Steam keyboard

| # | Check | Expected | Result |
|---|---|---|---|
| 7 | X opens search; the bottom bar shows ⧉ Steam keyboard | the hint is there (Gaming Mode only) | |
| 8 | Press View (⧉) in search | Steam's keyboard opens; typed letters appear in the search box | |
| 9 | Settings → Search keyboard → Steam, then X | Steam's keyboard opens with the search | |
| 10 | Enter on Steam's keyboard | search closes and shows the matches | |

## Installing (real changes: these install and remove things)

| # | Check | Expected | Result |
|---|---|---|---|
| 11 | Install a small Flathub app (Y → Install) | progress bar, then "Installed"; it appears in Installed and launches with A | |
| 12 | Uninstall it (Installed → Y → confirm) | it is gone; history (≡ in Downloads) shows both | |
| 13 | Install Proton-GE (Categories → Compatibility) | it appears in Steam → game → Properties → Compatibility after Steam restarts | |
| 14 | Install an AppImage (Categories → Emulator), then A → Make runnable | it appears in Add a Non-Steam Game and starts in Gaming Mode | |
| 15 | Turn Wi-Fi off during an install | "Offline" shows, the install waits and finishes when Wi-Fi is back | |

## Settings and system

| # | Check | Expected | Result |
|---|---|---|---|
| 16 | Text size → Largest; Motion → Reduced | all text is bigger and still readable; no sliding or moving background | |
| 17 | Suspend the Deck with Arxis Deck open, then wake it | the store still responds to the controller | |
| 18 | The clock in the top bar | shows the Deck's local time | |
| 19 | Restart Arxis Deck | settings and history are kept | |
