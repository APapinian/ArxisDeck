# Arxis Deck — Technical Architecture

> **Status:** Proposed baseline; validate dependencies and APIs during M0/M1.  
> **Architecture principle:** Rust application core, Qt Quick/QML presentation, provider-based integrations, user-space operation by default.

## 1. System overview

```text
┌──────────────────────────────────────────────────────────────┐
│                     Qt Quick / QML UI                        │
│ Gaming Mode • Desktop Mode • Pages • Components • Themes    │
└──────────────────────────────┬───────────────────────────────┘
                               │ typed commands / events
┌──────────────────────────────▼───────────────────────────────┐
│                    Application Facade                         │
│ navigation models • command validation • UI state            │
└──────────────────────────────┬───────────────────────────────┘
                               │
┌──────────────────────────────▼───────────────────────────────┐
│                       Rust Core                               │
│ Catalog • Install Manager • Update Manager • Queue            │
│ Library • Search • Settings • Diagnostics • Policy Engine     │
└─────────────┬────────────────┬────────────────┬───────────────┘
              │                │                │
       ┌──────▼─────┐   ┌──────▼─────┐   ┌──────▼────────┐
       │ Providers  │   │ Persistence│   │ OS Integration│
       │ Flatpak    │   │ SQLite     │   │ Steam paths   │
       │ AppImage   │   │ cache      │   │ D-Bus         │
       │ GitHub     │   │ history    │   │ notifications │
       │ Gaming     │   │ settings   │   │ launch helper │
       │ Distrobox* │   └────────────┘   └───────────────┘
       └────────────┘
```

`*` Optional advanced provider, not part of the initial release.

## 2. Technology decisions

| Area | Proposed technology | Notes |
|---|---|---|
| Core | Rust, stable toolchain | Business logic and provider orchestration |
| UI | Qt 6 Quick / QML | Controller-friendly, responsive UI |
| Rust ↔ Qt | Evaluate `cxx-qt` first | Prove toolchain and packaging in M1 |
| Async work | Tokio | Network, queues and subprocess coordination |
| Database | SQLite | Local metadata, cache, settings and history |
| HTTP | `reqwest` | GitHub API and downloads |
| Serialization | `serde`, JSON/TOML | Provider metadata and configuration |
| Logging | `tracing` | Structured diagnostics with redaction |
| Flatpak | `flatpak` CLI initially; D-Bus evaluation | Prefer supported machine-readable interfaces |
| Packaging | AppImage or user-space bundle after validation | Must be tested on actual SteamOS |
| Tests | Rust unit/integration tests + device tests | Real Deck testing remains mandatory |

These are implementation candidates, not a claim that all dependencies have already been validated on SteamOS. M0/M1 must verify current Qt, Rust, `cxx-qt`, compiler and runtime compatibility.

## 3. UI architecture

### 3.1 Shared application, adaptive presentation
- One application binary and one Rust core.
- Gaming Mode: full-screen, controller-first layout.
- Desktop Mode: resizable layout with mouse, keyboard and touch support.
- UI mode is a presentation profile, not a separate codebase.
- Detect screen size and input capabilities at runtime; allow manual override for testing.

### 3.2 QML structure

```text
ui/
├── Main.qml
├── App.qml
├── navigation/
│   ├── Router.qml
│   └── FocusController.qml
├── pages/
│   ├── DiscoverPage.qml
│   ├── SearchPage.qml
│   ├── CategoryPage.qml
│   ├── ApplicationPage.qml
│   ├── LibraryPage.qml
│   ├── DownloadsPage.qml
│   ├── UpdatesPage.qml
│   └── SettingsPage.qml
├── components/
│   ├── AppCard.qml
│   ├── SourceBadge.qml
│   ├── InstallButton.qml
│   ├── ProgressRow.qml
│   ├── ControllerHints.qml
│   └── ErrorPanel.qml
├── models/
└── themes/
```

### 3.3 Input model
- D-pad and left stick move focus.
- A activates/selects.
- B navigates back or closes a modal.
- X opens search where contextually appropriate.
- Y exposes secondary actions where shown in controller hints.
- Bumpers/triggers may switch top-level sections.
- Keyboard and mouse remain supported in Desktop Mode.
- Every interactive control must expose visible focus and accessible labels.

Do not hardcode Steam Input behavior based on assumptions. Validate actual controller events and Steam virtual keyboard behavior on hardware.

## 4. Rust core modules

```text
src/
├── main.rs
├── app/
│   ├── bootstrap.rs
│   ├── commands.rs
│   └── events.rs
├── core/
│   ├── catalog/
│   ├── search/
│   ├── library/
│   ├── operations/
│   ├── updates/
│   ├── policy/
│   ├── settings/
│   └── diagnostics/
├── providers/
│   ├── mod.rs
│   ├── flatpak/
│   ├── appimage/
│   ├── github/
│   ├── gaming/
│   └── distrobox/
├── persistence/
├── platform/
│   ├── steam_deck/
│   ├── filesystem/
│   └── process/
└── security/
```

Keep provider-specific code behind stable Rust traits. Avoid leaking Flatpak or GitHub implementation details into UI models.

## 5. Provider contract

Each provider should implement only capabilities it can genuinely support.

Conceptual interface:

```rust
trait SoftwareProvider {
    fn id(&self) -> ProviderId;
    fn capabilities(&self) -> ProviderCapabilities;

    async fn search(&self, query: SearchQuery)
        -> Result<Vec<SoftwareCandidate>, ProviderError>;

    async fn get_details(&self, id: SoftwareId)
        -> Result<SoftwareDetails, ProviderError>;

    async fn list_installed(&self)
        -> Result<Vec<InstalledSoftware>, ProviderError>;

    async fn install(&self, id: SoftwareId, options: InstallOptions)
        -> Result<OperationHandle, ProviderError>;

    async fn uninstall(&self, id: SoftwareId, options: UninstallOptions)
        -> Result<OperationHandle, ProviderError>;

    async fn check_updates(&self)
        -> Result<Vec<UpdateCandidate>, ProviderError>;
}
```

This is illustrative pseudocode. Adapt object safety, async-trait strategy, cancellation and event streaming to the selected Rust design. Do not force unsupported methods onto every provider; use capability declarations and optional provider-specific interfaces.

### 5.1 Flatpak provider
- Query installed refs and remotes using supported Flatpak interfaces.
- Use user installation scope by default.
- Avoid parsing human-oriented CLI output if machine-readable output or D-Bus APIs are available.
- Model app IDs, refs, architectures, branches, remotes and runtimes explicitly.
- Expose permission metadata where available.
- Track operation progress and provider errors.
- Do not mutate remotes without explicit user action.

### 5.2 AppImage provider
- Download into a staging directory under the user's data area.
- Validate expected file size and available digest/signature.
- Verify checksum before moving into the managed application directory.
- Do not assume that a checksum hosted on the same untrusted endpoint establishes publisher identity.
- Never execute during download or verification.
- Create desktop integration only after user confirmation.
- Store a manifest of files managed by Arxis Deck so uninstall does not delete unrelated data.

### 5.3 GitHub provider
- Use official GitHub Releases API.
- Support repository owner/name and release tag.
- Handle API rate limits, pagination, prereleases and missing releases.
- Asset selection must be explainable and based on architecture, OS and declared format.
- Prefer signed releases or checksums with a documented trust chain.
- Do not run release scripts or arbitrary post-install hooks.
- Treat a release as unverified when authenticity cannot be established.

### 5.4 Gaming tools provider
- Maintain a curated catalog of supported projects and installation routes.
- Prefer Flatpak or the project's documented SteamOS-compatible distribution method.
- Keep integration metadata separate from provider execution.
- Do not modify Steam library files without explicit confirmation and a recoverable plan.
- For compatibility tools, detect existing versions and avoid overwriting user data.
- Mark unsupported or untested Deck workflows clearly.

### 5.5 Distrobox provider (advanced)
- Detect whether Distrobox and a suitable container runtime are available.
- Create/manage only an explicitly configured container.
- Run package commands only inside that container.
- Never execute host `pacman`, host AUR helpers or host package installation commands.
- Explain container storage and exported desktop entries.
- Require user confirmation before creating, deleting or changing the container.
- Keep this provider disabled by default until M5 acceptance tests pass.

## 6. Operation model

All long-running operations should use a common lifecycle:

```text
Queued → Validating → Downloading → Verifying
       → Installing → Finalizing → Completed

Any stage may instead transition to:
Failed | Cancelled | NeedsUserAction
```

- Operations run off the UI thread.
- UI receives typed progress and state events.
- Persist operation state and diagnostic metadata.
- Cancellation is provider-specific; do not claim cancellation succeeded until confirmed.
- On application restart, reconcile interrupted operations with provider state.
- Retry only when the operation is known to be safe to retry.
- Use staging directories and atomic moves where practical.

## 7. Data model

Suggested logical entities:

- `Application`: stable internal ID, display name, summary, description, category.
- `Source`: provider ID, source identity, enabled state and configuration.
- `Release`: version/tag, architecture, asset metadata, verification state.
- `Installation`: provider-specific installed identity and observed version.
- `Operation`: operation type, lifecycle state, timestamps, result and redacted logs.
- `Download`: URL origin, target, bytes, expected size and verification outcome.
- `Favorite`: application ID and sort order.
- `Setting`: key/value with schema version.

SQLite is a cache and application-state store, not the authority for whether a package is installed. Reconcile installations from each provider on startup and refresh.

Use migrations. Store secrets in an OS-supported credential store if credentials become necessary; never store tokens in plaintext SQLite or logs.

## 8. Filesystem layout

Use XDG paths where possible:

```text
~/.local/share/arxis-deck/
├── arxis-deck.sqlite
├── downloads/
├── applications/
├── icons/
└── logs/

~/.config/arxis-deck/
└── config.toml

~/.cache/arxis-deck/
└── metadata/
```

AppImage executable storage may use `~/Applications/` if the user selects or accepts that location. Avoid writing to SteamOS system directories.

Never assume a fixed Steam library path. Discover candidate locations, validate them and ask the user to confirm before managing files there.

## 9. Security model

- Default to unprivileged user-space operation.
- No host `sudo`, host `pacman`, AUR on host, or `steamos-readonly disable`.
- Validate all user- and network-supplied paths.
- Prevent archive path traversal and symlink escape.
- Enforce reasonable download size limits and free-space checks.
- Validate HTTPS and redirect destinations.
- Verify signatures/checksums where possible and accurately represent confidence.
- No automatic execution of downloaded scripts.
- Require confirmation for installation, removal, permissions, remotes and destructive changes.
- Redact credentials, tokens and sensitive local paths from exported logs.
- Keep a clear ownership manifest for files managed by the application.

## 10. SteamOS integration

- Detect Steam Deck/SteamOS conservatively; support a manual compatibility mode.
- Read system information without changing the host.
- Use documented user-space integration routes.
- Provide a user-guided method to add Arxis Deck to Steam as a non-Steam application.
- Do not silently edit Steam's internal VDF/configuration files.
- Do not assume Gaming Mode can launch every desktop application correctly; test the actual launch path.
- Keep Desktop Mode usable if Gaming Mode integration is unavailable.

## 11. Packaging and updates

- First prove a reproducible development build on SteamOS.
- Select the distribution format only after validating Qt runtime dependencies and filesystem behavior.
- AppImage is a candidate, not a pre-decided guaranteed solution.
- Keep application updates separate from software-provider updates.
- Publish checksums and signed release artifacts when release infrastructure supports them.
- Updates must not overwrite user configuration, database or managed applications.
- Provide rollback to the previous Arxis Deck application version where feasible.

## 12. Observability and testing

- Structured logs using `tracing`.
- User-facing errors must explain the failure and next action.
- Diagnostic export must redact secrets.
- Unit tests for core models, policy and state transitions.
- Provider integration tests using fixtures/mocks.
- Filesystem safety tests using temporary directories.
- Steam Deck hardware tests for controller, keyboard, resolution, sleep/resume, network loss and OS updates.
