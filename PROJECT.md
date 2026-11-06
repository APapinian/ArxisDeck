# Arxis Deck — Project Definition

> **Project status:** Pre-development / specification  
> **Product type:** Steam Deck-focused software center  
> **Primary platform:** SteamOS on Valve Steam Deck  
> **Primary interaction:** Gamepad  
> **Primary display target:** 1280 × 800  
> **Project language:** English for code, identifiers and technical documentation; Turkish may be used in user-facing localization.

## 1. Product vision

Arxis Deck is a unified software and gaming-tools center designed for Steam Deck. It brings supported Linux applications, game launchers, compatibility tools, emulators and selected utilities into one controller-first interface.

The product should feel natural in SteamOS Gaming Mode while remaining fully usable in Desktop Mode. It must respect SteamOS's update model and must not depend on modifying the read-only host operating-system image.

Arxis Deck is inspired by the feature set and workflows of the Arxis project, but it is a new SteamOS-oriented application. Reuse of Arxis source code is not assumed. Before copying or adapting any code, inspect the exact repository, verify its license and notices, and document the compliance plan.

## 2. Goals

1. Make it easy to discover, install, update and remove supported software on Steam Deck.
2. Provide one catalog and one library across supported software sources.
3. Make all essential workflows usable with a controller, without requiring a keyboard or mouse.
4. Keep package operations transparent, cancellable where safe, observable and recoverable.
5. Preserve SteamOS host integrity: no routine host `pacman`, AUR, `sudo`, or read-only filesystem modification.
6. Support both Gaming Mode and Desktop Mode.
7. Keep providers modular so a failure or change in one source does not destabilize the whole application.
8. Clearly communicate source, version, permissions, download size and trust/verification status.

## 3. Non-goals

- Replacing Steam, SteamOS, Discover, or Valve's system update mechanism.
- Modifying SteamOS system images or disabling read-only protections.
- Running arbitrary install scripts from the internet without explicit informed consent.
- Promising that every Linux application or GitHub release is compatible with Steam Deck.
- Reimplementing Heroic, Lutris, Bottles, emulators, Proton or other third-party applications.
- Treating every GitHub repository or AppImage as trusted.
- Providing a general-purpose host package manager for SteamOS.

## 4. Target users and usage modes

### Gaming Mode
- Full-screen, controller-first layout.
- Large readable typography and focus indicators.
- D-pad, analog stick, A/B/X/Y, bumpers and triggers supported where appropriate.
- Steam virtual keyboard support for search and text entry.
- No essential action may require a mouse.

### Desktop Mode
- Resizable window.
- Mouse, keyboard, touch and controller support.
- Denser layouts and additional technical details.
- Same account-independent local catalog, installation state and settings as Gaming Mode.

## 5. Functional scope

### 5.1 Discovery and catalog
- Home/discovery page with featured and recently updated entries.
- Categories: applications, internet, multimedia, productivity, development, system utilities, game launchers, compatibility tools and emulators.
- Search by name, description, category, provider and application identifier.
- Filters by source, installed status, category and architecture where metadata exists.
- Application detail page with description, screenshots/icons where legally and technically available, version, source, publisher/project link, download size where known, permissions and verification status.
- Search results must distinguish similarly named packages and providers.

### 5.2 Supported source types

**Flatpak — primary provider**
- Discover installed and available applications.
- Support configured remotes, including Flathub when enabled by the user or installation configuration.
- Install, update and uninstall through supported Flatpak interfaces.
- Show runtime/dependency information where available.
- Show and explain application permissions where available.
- Respect user-level installation by default; do not silently change installation scope.

**AppImage**
- Download to a user-owned application directory, initially `~/Applications/`.
- Verify checksum and/or signature when trustworthy metadata is provided.
- Clearly label files for which no independent integrity/authenticity verification is available.
- Do not execute downloaded files automatically.
- Provide explicit install/registration and removal workflows.
- Do not silently grant executable permission or integrate desktop entries without user confirmation.

**GitHub Releases**
- Query public GitHub Releases through the official API where practical.
- Select release assets using explicit, explainable compatibility rules.
- Prefer assets with checksums or signatures published by the project.
- Display repository, release tag, publication date, asset name, size and verification status.
- Never execute release-provided scripts automatically.
- Handle rate limits, unavailable releases, prereleases and architecture mismatch.
- User-provided repositories may be added, but must be visibly marked as user-added sources.

**Gaming tools**
- Catalog and manage existing third-party tools through their supported distribution channels.
- Candidate integrations include Heroic Games Launcher, Lutris, Bottles, ProtonUp-Qt, emulators and related utilities.
- Do not bundle, fork or impersonate these projects.
- Proton compatibility-tool installation must use a verified and documented mechanism and must not overwrite user-installed versions without consent.
- Treat each integration as a separate capability with its own support and test status.

**Distrobox / isolated Arch environment — advanced, optional**
- Investigate support for an isolated Arch container as an advanced feature.
- AUR and `pacman` operations, if implemented, must run inside the container and never against the SteamOS host.
- Explain container storage, lifecycle, exported applications and maintenance requirements.
- Do not make this a prerequisite for normal store operation.
- Do not promise full AUR compatibility until tested on supported SteamOS versions.

### 5.3 Library and operations
- Unified installed-applications view with provider and installed version.
- Install, update and uninstall queues.
- Progress, current phase, estimated size where available, logs and actionable errors.
- Operation history with timestamps and outcomes.
- Cancellation only where the underlying operation supports safe cancellation.
- Retry for recoverable failures.
- Favorites and locally maintained collections.
- Refresh installed state from providers; SQLite cache is not the source of truth for installed state.

### 5.4 Updates
- Check for updates on demand and optionally on a user-configurable schedule.
- Show available updates before applying them.
- Allow individual updates and an explicitly requested update-all operation.
- Never update Proton compatibility tools or other sensitive gaming components silently.
- Preserve provider-specific version and rollback limitations in the UI.

### 5.5 Settings and maintenance
- Enable/disable supported sources.
- Manage Flatpak remotes through safe, explicit workflows.
- Configure download/cache locations where supported.
- Configure update checks and notifications.
- Export diagnostic logs with secrets and personal paths redacted where practical.
- Provide a troubleshooting view and environment diagnostics.
- Do not offer a button that disables SteamOS read-only protections.

## 6. Safety and trust requirements

- No host `pacman` or AUR execution.
- No routine `sudo` or privilege escalation.
- No `steamos-readonly disable`.
- No modification of SteamOS system files or boot configuration.
- Never run downloaded scripts as part of a normal install.
- Use HTTPS for network downloads.
- Validate URLs, redirects, file sizes and archive paths.
- Prevent path traversal and unsafe archive extraction.
- Verify checksums/signatures when available and accurately label the absence of verification.
- Do not present a checksum hosted beside an asset as proof of publisher identity unless the trust model supports that claim.
- Ask before installing, removing, changing remotes, granting permissions or deleting user data.
- Display the exact source and target installation scope before a destructive or security-relevant action.
- Keep operation logs useful but avoid storing access tokens or secrets.
- Use least privilege and user-owned directories by default.

## 7. UX and accessibility requirements

- 1280 × 800 is the baseline viewport, not the only supported size.
- Minimum readable text sizes and high-contrast focus states.
- Clear focus order and no focus traps.
- Consistent controller hints.
- Back navigation must be predictable.
- Search must support the Steam virtual keyboard.
- Loading, empty, offline, error and success states must be designed.
- Long-running operations must not freeze the interface.
- Respect reduced-motion preference where practical.
- Do not rely on color alone to communicate status.

## 8. Release principles

- Start with an experimental, clearly labelled build.
- Publish checksums and signed release artifacts when release infrastructure is ready.
- Keep a compatibility matrix by SteamOS version, Deck hardware generation and Desktop/Gaming Mode.
- Do not label a release stable until installation, update, removal, recovery and OS-update regression tests pass on real hardware.
- Provide a documented uninstall process that removes only Arxis Deck's own files and explicitly selected managed applications.

## 9. Definition of done

A feature is complete only when:
1. Its behavior is implemented and documented.
2. Its success, failure, cancellation and offline states are handled.
3. Relevant automated tests pass.
4. It has been tested on a Steam Deck or a clearly documented equivalent environment.
5. It does not violate the host-integrity and safety requirements.
6. Its UI is usable with a controller in Gaming Mode.
7. Its limitations and provider-specific behavior are visible to the user.
