# Arxis Deck — Development Roadmap

> **Status:** Proposed plan  
> **Milestones:** M0–M7  
> **Rule:** A milestone is not complete because code compiles. It is complete only when its acceptance criteria and relevant tests pass.

## Delivery principles

- Work in small, reviewable increments.
- Keep `main` buildable.
- Do not let an AI coding agent make broad, unreviewed changes.
- Ask the agent to inspect before editing.
- Require a summary of changed files, commands run, test results and known limitations after each task.
- Never test destructive package operations on the SteamOS host without a documented, recoverable test plan.
- Use mocks or a disposable environment for early provider tests; validate final behavior on real Steam Deck hardware.

## M0 — Repository and feasibility research

**Objective:** Understand Arxis and validate the proposed technical direction before implementation.

### Tasks
1. Inspect the complete Arxis repository and record its structure.
2. Verify repository identity, license file, copyright notices and dependency licenses.
3. Map existing package providers and their command execution paths.
4. Identify code that is reusable, code that requires redesign and code that must not be carried over.
5. Verify current SteamOS constraints and supported user-space installation mechanisms.
6. Prototype-build the proposed Rust/Qt/QML toolchain in a controlled environment.
7. Record unresolved questions and risks.

### Deliverables
- `docs/research/arxis-analysis.md`
- `docs/research/license-review.md`
- `docs/research/steamos-compatibility.md`
- `docs/architecture-decisions/ADR-0001-technology-stack.md`
- Updated `ARCHITECTURE.md`

### Tests / checks
- Repository and license findings include file paths or source references.
- No assumptions about host pacman/AUR are treated as facts.
- Minimal Qt/QML + Rust proof of concept builds.
- A basic QML screen can receive a typed event from Rust.

### Acceptance criteria
- The license and reuse decision is documented.
- The selected UI/core bridge is proven in a minimal build or replaced with a documented alternative.
- Critical SteamOS constraints are documented.
- No production feature implementation begins before review.

## M1 — Application skeleton and controller-first shell

**Objective:** Establish a working application shell on Steam Deck.

### Tasks
1. Create Rust workspace and module boundaries.
2. Create Qt Quick/QML application shell.
3. Add application bootstrap, structured logging and configuration loading.
4. Add SQLite connection and migration framework.
5. Implement navigation and adaptive Gaming/Desktop presentation profiles.
6. Implement controller focus, visible hints and keyboard/mouse support.
7. Add a diagnostics/about page.

### Tests / checks
- Unit tests for configuration and database migrations.
- Application starts with a clean profile and an existing profile.
- No UI freeze during simulated background work.
- Navigation works with controller, keyboard and mouse.
- Test at 1280×800 and a resizable desktop window.

### Acceptance criteria
- Runs on target SteamOS hardware or a clearly documented equivalent.
- Main navigation is usable without mouse or keyboard.
- No package operation or privileged command is invoked.
- Logs and configuration are written only to user-owned locations.

## M2 — Flatpak provider and first usable store

**Objective:** Deliver the first end-to-end software installation workflow.

### Tasks
1. Detect Flatpak availability and installed applications.
2. Query configured remotes and available catalog metadata.
3. Implement search, category browsing and application detail pages.
4. Implement user-scope installation by default.
5. Implement progress, errors, cancellation where supported and operation history.
6. Implement update checks and user-confirmed updates.
7. Implement uninstall with explicit handling of user data and dependencies.

### Tests / checks
- Provider tests with fixtures and a test remote where practical.
- Install, update and remove a harmless test application.
- Test offline, rate-limited, missing remote, insufficient disk space and interrupted operation scenarios.
- Confirm host system files and read-only protections are untouched.
- Confirm installed state is refreshed from Flatpak rather than inferred only from SQLite.

### Acceptance criteria
- User can search, inspect, install, update and uninstall a Flatpak from Gaming Mode.
- User can see source, scope, version and relevant permissions before installation.
- Errors are actionable and operations are recorded.
- No host pacman, AUR or sudo is used.

## M3 — AppImage and GitHub Releases

**Objective:** Add non-Flatpak software with explicit trust and integrity handling.

### Tasks
1. Implement GitHub Releases metadata provider.
2. Implement architecture and asset-format matching.
3. Implement download manager with pause/resume only where safely supported.
4. Implement staging, checksum/signature verification and safe finalization.
5. Implement AppImage registration and removal using a managed-file manifest.
6. Add source trust and verification labels to UI.
7. Add user-managed GitHub repository entries.

### Tests / checks
- Missing checksum, invalid checksum, invalid signature and mismatched architecture.
- Redirect handling, rate limits, interrupted downloads and disk-full conditions.
- Malicious archive path traversal fixtures.
- Confirm downloaded files are not executed automatically.
- Confirm uninstall removes only files listed in the Arxis Deck ownership manifest.

### Acceptance criteria
- AppImages and supported GitHub assets can be downloaded and managed safely.
- Verification status is accurate and visible.
- No unverified file is described as publisher-authenticated.
- No install script is automatically executed.

## M4 — Gaming tools and compatibility utilities

**Objective:** Make the store useful for gaming-specific workflows.

### Candidate catalog
- Heroic Games Launcher
- Lutris
- Bottles
- ProtonUp-Qt
- Emulators and related utilities
- Other Deck-relevant tools after compatibility review

### Tasks
1. Define curated catalog metadata and support status.
2. Map each tool to a supported installation source.
3. Detect already-installed tools and avoid duplicate installs.
4. Investigate Proton compatibility-tool discovery and installation paths.
5. Add explicit confirmation and preflight checks for compatibility-tool changes.
6. Add explanatory pages for tool purpose, source and limitations.

### Tests / checks
- Verify source and package identity for each catalog item.
- Confirm existing user installations are detected.
- Confirm no tool overwrites existing compatibility data without consent.
- Validate launch and basic operation in Gaming Mode and Desktop Mode where applicable.

### Acceptance criteria
- Every catalog entry has a verified source and documented support status.
- No unsupported integration is presented as fully supported.
- Proton-related operations are reversible where the underlying mechanism permits.

## M5 — Optional Distrobox / isolated Arch and AUR

**Objective:** Offer advanced package workflows without touching the SteamOS host.

### Tasks
1. Research Distrobox and container-runtime availability on target SteamOS.
2. Design explicit container creation, inspection, update and removal.
3. Implement a provider that can execute commands only inside the configured container.
4. Add AUR support only after container isolation is proven.
5. Manage exported application entries and their ownership.
6. Explain disk usage, maintenance and security implications.

### Tests / checks
- Assert command execution target is the container, never host.
- Test container unavailable, broken container, failed package build and disk exhaustion.
- Test exported app launch and clean removal.
- Test container deletion confirmation and data-loss warning.
- Verify host package database and system image remain unchanged.

### Acceptance criteria
- Feature is disabled by default until all safety tests pass.
- All package operations are isolated.
- No host `pacman`, host AUR helper or host privilege escalation path exists.
- Limitations are prominently documented.

## M6 — Gaming Mode polish and user experience

**Objective:** Make the complete application comfortable to use as a handheld console experience.

### Tasks
1. Refine home page, featured content and category browsing.
2. Refine controller focus and navigation transitions.
3. Integrate Steam virtual keyboard for text entry.
4. Add clear download queue and operation overlays.
5. Improve offline, empty, loading, error and recovery states.
6. Add accessibility options, scalable text and reduced-motion behavior.
7. Test suspend/resume, display changes and input reconnection.

### Tests / checks
- Complete primary user journeys with controller only.
- No keyboard/mouse required for search, install, update or uninstall.
- Focus is always visible and never trapped.
- UI remains responsive during downloads and package operations.
- Test 1280×800, Desktop Mode resizing and touch input.

### Acceptance criteria
- All core workflows are controller-complete.
- User can understand the current state of every long-running operation.
- UI works in Gaming Mode and Desktop Mode without separate application builds.

## M7 — Packaging, release engineering and stability

**Objective:** Prepare a reliable, maintainable distribution.

### Tasks
1. Select and validate final packaging format.
2. Build reproducibly in documented CI or build environment.
3. Add release checksums and signing where feasible.
4. Implement safe application self-update or a documented update route.
5. Add diagnostics export with secret redaction.
6. Add uninstall and data-retention controls.
7. Run SteamOS update regression tests.
8. Publish compatibility matrix and known issues.

### Tests / checks
- Fresh install, upgrade, downgrade/rollback where supported and uninstall.
- Existing user data preserved across application updates.
- SteamOS update followed by launch and provider smoke tests.
- Verify checksums/signatures and release provenance.
- Verify no system partition modification.

### Acceptance criteria
- Release artifact installs and launches on supported Steam Deck configuration.
- Update and uninstall paths are documented and tested.
- Critical operations pass on real hardware.
- Known limitations and compatibility matrix are published.
- Release is labelled experimental/beta until stability evidence justifies a stable designation.

## Cross-milestone quality gates

Every milestone must pass:

1. `cargo fmt --check`
2. `cargo clippy` with project-approved warnings policy
3. `cargo test`
4. QML lint/build checks selected for the actual Qt toolchain
5. Dependency/license review for new dependencies
6. Security review for filesystem, process and network changes
7. Manual controller usability check for UI changes
8. A concise change report and updated documentation

## AI coding-agent workflow

For Claude Code or another coding agent:

1. Read `PROJECT.md`, `ARCHITECTURE.md` and this roadmap before changing files.
2. Read the current milestone's tasks and acceptance criteria.
3. Inspect the existing repository and report the planned file changes.
4. Implement only the requested milestone slice.
5. Run the relevant tests and report exact commands and outcomes.
6. Do not claim a test passed if it was not run.
7. Do not make unrelated refactors.
8. Do not run destructive commands or alter host SteamOS configuration.
9. Stop and ask for review when a decision changes architecture, security policy, license strategy or data layout.

## Current project state

- [ ] M0 — Research and feasibility
- [ ] M1 — Application skeleton
- [ ] M2 — Flatpak store
- [ ] M3 — AppImage and GitHub
- [ ] M4 — Gaming tools
- [ ] M5 — Optional isolated Arch/AUR
- [ ] M6 — Gaming Mode polish
- [ ] M7 — Packaging and release
