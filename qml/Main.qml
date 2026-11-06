// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Arxis Deck, M1 test window.
// Designed at 1280x800 (Steam Deck), scaled uniformly on other screens.
// All navigation goes through handleAction(name, source); the controller (from Rust),
// the keyboard and touch all call it, so every screen is reachable without a mouse.
//
// Motion follows the Steam store / Big Picture feel: a focus ring that glides between
// cards, a background tinted by the focused item, cards that cascade in on section
// change, a details page that zooms out of the card, and a short boot splash.
// Only plain Qt Quick items and animations are used (no shaders), so it also runs on
// the software renderer used by the automated checks.

import QtQuick
import QtQuick.Window
import "components"
import "pages"
import "themes"

Window {
    id: root
    width: 1280
    height: 800
    minimumWidth: 640
    minimumHeight: 400
    visible: false
    title: "Arxis Deck"
    color: theme.bg

    // ---- State (read by the Rust self-test) ----
    readonly property var tabs: ["Discover", "Installed", "Categories", "Downloads", "Settings"]
    readonly property var tabCounts: [24, 8, 12, 4, 6]
    // Categories in catalogue order (sample items: the six base categories).
    readonly property var categories: {
        var seen = []
        for (var i = 0; i < catalog.length; i++)
            if (seen.indexOf(catalog[i].category) < 0) seen.push(catalog[i].category)
        return seen
    }
    readonly property int columns: 4
    property int tabIndex: 0
    property var tabFocus: [0, 0, 0, 0, 0]
    readonly property int focusIndex: grid.currentIndex
    readonly property bool detailOpen: detail.opened
    property int detailButton: 0
    property string lastAction: ""
    property string lastSource: ""
    property string controllerStatus: "Looking for controllers…"
    property string controllerConnected: "false"
    readonly property bool toastVisible: toast.shown
    readonly property real gridContentY: grid.contentY - grid.originY
    readonly property bool hudReady: topBar.visible && topBar.height > 0
                                     && hintBar.visible && hintBar.height > 0
                                     && topBar.tabCount === tabs.length
                                     && hintBar.hintCount > 0
                                     && grid.count > 0 && hintBar.statusLine.length > 0
    readonly property string hudReport: "tabs=" + topBar.tabCount + ", hints=" + hintBar.hintCount
                                        + ", cards=" + grid.count + ", status='" + hintBar.statusLine + "'"

    // ---- Motion state ----
    property bool introDone: false
    property bool staggering: true      // new cards cascade in while this is set
    property int staggerTopRow: 0       // first visible row after a section change
    property int slideDir: 1            // +1 when moving to a later section, -1 earlier
    readonly property real focusHue: hueFor(currentItemNo + tabIndex * 7)
    signal edgeBump(int dx, int dy)

    // ---- Search (Y) and options menu (Start) ----
    property bool searchOpen: false
    property string searchQuery: ""
    readonly property int searchMaxLength: 24
    property int kbRow: 1
    property int kbCol: 0
    property bool menuOpen: false
    property int menuIndex: 0
    readonly property int resultCount: grid.count
    // Each key: [label, value, width in key units]. Every row is 10 units wide.
    readonly property var kbRows: [
        [["1","1",1],["2","2",1],["3","3",1],["4","4",1],["5","5",1],["6","6",1],["7","7",1],["8","8",1],["9","9",1],["0","0",1]],
        [["q","q",1],["w","w",1],["e","e",1],["r","r",1],["t","t",1],["y","y",1],["u","u",1],["i","i",1],["o","o",1],["p","p",1]],
        [["a","a",1],["s","s",1],["d","d",1],["f","f",1],["g","g",1],["h","h",1],["j","j",1],["k","k",1],["l","l",1],["-","-",1]],
        [["z","z",1],["x","x",1],["c","c",1],["v","v",1],["b","b",1],["n","n",1],["m","m",1],[".",".",1],["'","'",1],["⌫","BKSP",1]],
        [["Space","SPACE",5],["Clear","CLEAR",2],["Done","DONE",3]]
    ]
    readonly property var menuItems: ["Open details", "Search",
                                      queueStatus(tabIndex, currentItemNo) === "" ? "Add to Downloads" : "Remove from Downloads",
                                      "About this test build", "Close"]

    // ---- Store backend (set by Rust) ----
    // "flatpak": real Flathub apps installed for this user; "simulated": sample items, nothing installed.
    property string backendMode: "simulated"
    property string backendStatus: ""
    readonly property bool realMode: backendMode === "flatpak"
    property string catalogJson: ""                // Rust: starter list (real mode only)
    property string installedJson: "[]"            // Rust: [{id, name, version}] installed for this user
    property string updatesJson: "[]"              // Rust: [{id, version}] installed apps with an update
    property string historyJson: "[]"              // Rust: recent operations from arxis-deck.sqlite, newest first
    readonly property var historyOps: { try { return JSON.parse(historyJson) } catch (e) { return [] } }
    readonly property int historyCount: historyOps.length
    property bool historyOpen: false
    property int historyIndex: 0

    function handleHistoryAction(name) {
        switch (name) {
        case "up": if (historyIndex > 0) historyIndex--; else edgeBump(0, -1); break
        case "down": if (historyIndex < historyCount - 1) historyIndex++; else edgeBump(0, 1); break
        case "page_up": historyIndex = Math.max(0, historyIndex - 6); break
        case "page_down": historyIndex = Math.max(0, Math.min(historyCount - 1, historyIndex + 6)); break
        case "back": case "menu": historyOpen = false; break
        }
    }

    // About & diagnostics (Settings → last row). Rust fills diagnosticsJson (src/diagnostics.rs).
    property bool aboutOpen: false
    property string diagnosticsJson: "[]"
    readonly property var aboutSections: { try { return JSON.parse(diagnosticsJson) } catch (e) { return [] } }
    property string exportPath: ""

    function openAbout() {
        sendCommand("diagnostics")
        aboutOpen = true
    }

    function handleAboutAction(name) {
        switch (name) {
        case "up": aboutView.flick.contentY = Math.max(0, aboutView.flick.contentY - 80); break
        case "down": aboutView.flick.contentY = Math.min(Math.max(0, aboutView.flick.contentHeight - aboutView.flick.height), aboutView.flick.contentY + 80); break
        case "secondary":
            showToast("Saving the diagnostics report…")
            sendCommand("export-diagnostics")
            break
        case "back": case "menu": aboutOpen = false; break
        }
    }

    function historyVerb(kind) {
        return ({ install: "Installed", update: "Updated", uninstall: "Uninstalled", launch: "Launched" })[kind] || kind
    }
    readonly property var updates: { try { return JSON.parse(updatesJson) } catch (e) { return [] } }
    readonly property int updateCount: updates.length
    function hasUpdate(id) {
        for (var i = 0; i < updates.length; i++) if (updates[i].id === id) return true
        return false
    }
    property string commands: ""                   // to Rust: "verb<TAB>arg[<TAB>arg2]" lines; Rust empties it
    property string removingName: ""
    property string lastLaunched: ""                // app ID of the last app started (read by the self-test)

    function appName(id) {
        for (var i = 0; i < installedApps.length; i++) if (installedApps[i].id === id) return installedApps[i].name
        for (var j = 0; j < catalog.length; j++) if (catalog[j].id === id) return catalog[j].name
        return id
    }

    function isCompat(id) { return id.indexOf("compat:") === 0 }
    function itemSource(no) { return catalog[no] && catalog[no].source ? catalog[no].source : "flatpak" }

    function launchApp(id) {
        if (isCompat(id)) {
            showToast("Choose " + appName(id) + " in a game's Properties → Compatibility (restart Steam first)")
            return
        }
        sendCommand("launch", id)
    }

    // Replaces the catalogue while keeping the focused card focused.
    function setCatalog(next) {
        var keepId = grid.count > 0 ? itemId(currentItemNo) : ""
        catalog = next
        Qt.callLater(function () {
            for (var i = 0; i < visibleItems.length; i++)
                if (itemId(visibleItems[i]) === keepId) { grid.currentIndex = i; return }
        })
    }

    // Proton-GE cards arrive through "catalog-add"; Rust picks the release assets (src/github.rs).
    readonly property var baseCategories: ["Tool", "Emulator", "Compatibility", "Utility", "Launcher", "Plugin"]
    property var catalog: sampleCatalog()          // [{id, name, category, summary}]
    onCatalogJsonChanged: {
        if (catalogJson === "") return
        try { catalog = JSON.parse(catalogJson) } catch (e) { console.warn("catalog:", e) }
    }
    readonly property var installedApps: { try { return JSON.parse(installedJson) } catch (e) { return [] } }

    function sampleCatalog() {
        var out = []
        for (var i = 0; i < 24; i++)
            out.push({ id: "sample.item" + (i + 1), name: "Sample item " + (i + 1),
                       category: baseCategories[i % baseCategories.length], summary: "" })
        return out
    }

    // One command line: verb, then any arguments, separated by tabs.
    // Connection to Flathub (Rust checks it; every 30 s while offline).
    property string networkState: "unknown"   // "online" | "offline" | "unknown"
    property string networkMessage: ""
    // Flathub search: "" | "searching" | "ok:<n>" | an error message.
    property string searchStatus: ""
    readonly property string searchStatusText: !realMode || searchQuery.trim().length < 2 || searchStatus === "" ? ""
        : searchStatus === "searching" ? "Searching Flathub…"
        : searchStatus.indexOf("ok:") === 0 ? searchStatus.slice(3) + " results from Flathub"
        : searchStatus

    function sendCommand(verb) {
        var parts = [verb]
        for (var i = 1; i < arguments.length; i++) parts.push(arguments[i] === undefined ? "" : String(arguments[i]))
        commands = commands + parts.join("\t") + "\n"
    }

    function isInstalledId(id) {
        for (var i = 0; i < installedApps.length; i++)
            if (installedApps[i].id === id) return true
        return false
    }

    // Real mode: searching also asks Flathub; new results are added to the catalogue.
    onSearchQueryChanged: if (realMode && searchQuery.trim().length >= 2) searchDebounce.restart()
    Timer {
        id: searchDebounce
        interval: 500
        onTriggered: {
            root.searchStatus = root.networkState === "offline" ? "Offline: showing only what is already listed" : "searching"
            if (root.networkState !== "offline") root.sendCommand("search", root.searchQuery.trim())
        }
    }

    // Events from Rust: progress / done / installed / removed / catalog-add.
    function backendEvent(kind, a, b) {
        var i
        switch (kind) {
        case "progress":
            i = queueFindId(a)
            if (i >= 0 && queueModel.get(i).status === "downloading") {
                queueModel.setProperty(i, "progress", Number(b))
                queueRev++
            }
            break
        case "done":
            i = queueFindId(a)
            if (i < 0) break
            var q = queueModel.get(i)
            if (b === "ok") {
                queueModel.setProperty(i, "status", "done")
                queueModel.setProperty(i, "progress", 1)
                showToast(q.name + (q.op === "update" ? " is up to date"
                                    : q.op === "install-compat" ? " is installed · restart Steam, then choose it in a game's Properties → Compatibility"
                                    : q.op === "install-appimage" ? " saved to ~/Applications · in Installed, press A to make it runnable"
                                    : realMode ? " is installed" : " is ready (simulated)"))
            } else if (b === "cancelled") {
                if (q.status === "downloading") queueModel.setProperty(i, "status", "paused")
            } else {
                queueModel.setProperty(i, "status", "failed")
                queueModel.setProperty(i, "error", b)
                showToast(q.name + ": " + b)
            }
            queueRev++
            break
        case "installed":
            installedJson = a
            // Finished downloads that are no longer installed leave Downloads too.
            for (i = queueModel.count - 1; i >= 0; i--)
                if (queueModel.get(i).status === "done" && !isInstalledId(queueModel.get(i).id))
                    queueModel.remove(i)
            if (downloadsIndex >= queueModel.count) downloadsIndex = Math.max(0, queueModel.count - 1)
            queueRev++
            break
        case "removed":
            showToast(b === "ok" ? removingName + " uninstalled" + (realMode ? "" : " (simulated)")
                                 : "Couldn't uninstall: " + b)
            break
        case "history":
            historyJson = a
            break
        case "diagnostics":
            diagnosticsJson = a
            break
        case "steam-keyboard":
            // a: "opened" (asked Steam) or "test" (automated run, Steam not called); b: error.
            steamKeyboardRequests++
            if (b !== "") showToast("Steam's keyboard didn't open: " + b + " · use the keys on screen")
            break
        case "exported":
            exportPath = a
            showToast(a !== "" ? "Report saved: " + a : "Couldn't save the report: " + b)
            break
        case "details":
            var dm = Object.assign({}, detailsMap)
            try { dm[a] = JSON.parse(b) } catch (e) { dm[a] = { error: String(e), permissions: [] } }
            detailsMap = dm
            break
        case "registered":
            showToast(b === "ok" ? appName(a) + " can now be started · add it to Steam with \"Add a Non-Steam Game\""
                                 : "Couldn't make it runnable: " + b)
            break
        case "updates":
            updatesJson = a
            break
        case "network":
            var was = networkState
            networkState = a
            networkMessage = b
            if (was === "offline" && a === "online") showToast("Back online · waiting downloads continue")
            else if (a === "offline") showToast("Offline · " + b)
            break
        case "search-done":
            if (a === searchQuery.trim()) searchStatus = b
            break
        case "notice":
            showToast(a)
            break
        case "launched":
            if (b === "ok") {
                lastLaunched = a
                showToast("Starting " + appName(a) + (realMode ? "…" : " (simulated)"))
            } else {
                showToast("Couldn't start " + appName(a) + ": " + b)
            }
            break
        case "catalog-add":
            try {
                var add = JSON.parse(a), next = catalog.slice(), known = {}
                for (i = 0; i < next.length; i++) known[next[i].id] = true
                for (i = 0; i < add.length; i++)
                    if (!known[add[i].id]) { next.push(add[i]); known[add[i].id] = true }
                if (next.length !== catalog.length) setCatalog(next)
            } catch (e) { console.warn("catalog-add:", e) }
            break
        }
    }

    // ---- Downloads (fourth section) ----
    readonly property int downloadsTab: 3
    readonly property bool inDownloads: root.tabIndex === root.downloadsTab
    property int downloadsIndex: 0
    property int queueRev: 0                        // bumped on every queue change, for bindings
    readonly property int queueCount: (queueRev, queueModel.count)
    function countStatus(list) {
        queueRev
        var n = 0
        for (var i = 0; i < queueModel.count; i++) if (list.indexOf(queueModel.get(i).status) >= 0) n++
        return n
    }
    readonly property int doneCount: countStatus(["done"])
    readonly property int activeCount: countStatus(["queued", "downloading", "paused"])
    readonly property int downloadingCount: countStatus(["downloading"])
    readonly property string headerSubtitle: header.subtitle   // read by the self-test
    readonly property string selectedDownloadStatus: { queueRev; return downloadsIndex < queueModel.count ? queueModel.get(downloadsIndex).status : "" }
    readonly property real activeProgress: {
        queueRev
        for (var i = 0; i < queueModel.count; i++)
            if (queueModel.get(i).status === "downloading") return queueModel.get(i).progress
        return 0
    }

    ListModel { id: queueModel }    // roles: tab, no, id, name, category, progress, status

    function queueFind(tab, no) { return queueFindId(itemId(no)) }

    function queueFindId(id) {
        for (var i = 0; i < queueModel.count; i++)
            if (queueModel.get(i).id === id) return i
        return -1
    }

    // "" when not in Downloads, else queued / downloading / paused / failed / done.
    function queueStatus(tab, no) {
        queueRev
        var i = queueFind(tab, no)
        return i < 0 ? "" : queueModel.get(i).status
    }

    function queueProgress(tab, no) {
        queueRev
        var i = queueFind(tab, no)
        return i < 0 ? 0 : queueModel.get(i).progress
    }

    function toggleDownload(tab, no) {
        var i = queueFind(tab, no)
        if (i >= 0) {
            removeDownload(i)
            return
        }
        if (isInstalledId(itemId(no))) {
            showToast(itemName(no) + " is already installed · uninstall it from Installed")
            return
        }
        var src = itemSource(no), compat = src === "compat", appimg = src === "appimage", it = catalog[no]
        queueModel.append({ tab: tab, no: no, id: itemId(no), name: itemName(no), category: categoryOf(tab, no), icon: itemIcon(no),
                            op: compat ? "install-compat" : appimg ? "install-appimage" : "install",
                            url: compat || appimg ? it.url : "", sum: compat || appimg ? it.sum : "",
                            // free space: Proton unpacks to ~3× the download; an AppImage needs its size plus staging
                            need: compat && it.size ? Math.ceil(it.size * 3 / 1024) : appimg && it.size ? Math.ceil(it.size * 1.1 / 1024) : 0,
                            file: appimg ? it.file : "", size: appimg ? it.size : 0,
                            version: appimg ? it.version : "", repo: appimg ? it.repo : "",
                            error: "", progress: 0, status: "queued" })
        queueRev++
        showToast(itemName(no) + " added to Downloads" + (realMode ? "" : " (simulated)"))
    }

    function removeDownload(i) {
        var q = queueModel.get(i)
        var name = q.name
        if (q.status === "downloading") sendCommand("cancel", q.id)
        queueModel.remove(i)
        if (downloadsIndex >= queueModel.count) downloadsIndex = Math.max(0, queueModel.count - 1)
        queueRev++
        showToast(name + " removed from Downloads")
    }

    // A in Downloads: pause/resume a waiting item, stop a running one, retry a failed one.
    function togglePause(i) {
        var q = queueModel.get(i)
        var was = q.status
        if (was === "done") { launchApp(q.id); return }
        if (was === "downloading") sendCommand("cancel", q.id)
        queueModel.setProperty(i, "status", was === "paused" || was === "failed" ? "queued" : "paused")
        if (was === "downloading" || was === "failed") queueModel.setProperty(i, "progress", 0)
        queueRev++
    }

    // Starts the next waiting item when nothing is downloading (one at a time, in order).
    Timer {
        id: downloadTimer
        interval: 100
        repeat: true
        running: root.activeCount > 0
        onTriggered: {
            // Offline: waiting items stay queued and start by themselves when the connection is back.
            if (root.networkState === "offline") return
            for (var i = 0; i < queueModel.count; i++)
                if (queueModel.get(i).status === "downloading") return
            for (var j = 0; j < queueModel.count; j++) {
                var q = queueModel.get(j)
                if (q.status === "queued") {
                    queueModel.setProperty(j, "status", "downloading")
                    queueModel.setProperty(j, "progress", 0)
                    root.sendCommand(q.op, q.id, q.name, q.url, q.sum, q.need, q.file, q.size, q.version, q.repo)
                    root.queueRev++
                    return
                }
            }
        }
    }

    function handleDownloadsAction(name) {
        switch (name) {
        case "up": if (downloadsIndex > 0) downloadsIndex--; else edgeBump(0, -1); return true
        case "down": if (downloadsIndex < queueModel.count - 1) downloadsIndex++; else edgeBump(0, 1); return true
        case "left": case "right": edgeBump(name === "left" ? -1 : 1, 0); return true
        case "page_up": downloadsIndex = 0; return true
        case "page_down": downloadsIndex = Math.max(0, queueModel.count - 1); return true
        case "confirm":
            if (queueModel.count > 0) togglePause(downloadsIndex)
            else showToast("Press Y on any item to add it to Downloads")
            return true
        case "secondary":
            if (queueModel.count > 0) removeDownload(downloadsIndex)
            return true
        case "menu":
            historyIndex = 0
            historyOpen = true
            return true
        case "search": case "view":
            showToast("Not available in Downloads")
            return true
        }
        return false
    }

    // ---- Settings (last section) ----
    readonly property int settingsTab: 4
    readonly property bool inSettings: root.tabIndex === root.settingsTab
    // Dark themes from the Arxis theme table: [background, accent].
    readonly property var themeDefs: ({
        "Arch Midnight": ["#070b14", "#f97316"],
        "Ocean Depth": ["#060d1a", "#f97316"],
        "Tokyo Night": ["#1a1b2e", "#ff007c"],
        "Catppuccin Mocha": ["#1e1e2e", "#cba6f7"],
        "Nord Arctic": ["#2e3440", "#88c0d0"],
        "Gruvbox Warm": ["#282828", "#fabd2f"],
        "Dracula Pro": ["#22212c", "#ff79c6"],
        "Everforest": ["#1e2326", "#a7c080"]
    })
    readonly property var settingsRows: [
        { key: "theme", label: "Theme", desc: "Colours from the Arxis theme collection",
          options: ["Arch Midnight", "Ocean Depth", "Tokyo Night", "Catppuccin Mocha", "Nord Arctic", "Gruvbox Warm", "Dracula Pro", "Everforest"] },
        { key: "repeat", label: "Hold-to-repeat speed", desc: "How fast focus moves while a direction is held",
          options: ["Slow", "Normal", "Fast"] },
        { key: "clock", label: "Clock", desc: "Time shown in the top bar", options: ["24-hour", "12-hour"] },
        { key: "hints", label: "Button hints", desc: "What each button does, along the bottom", options: ["Show", "Hide"] },
        { key: "glow", label: "Background glow", desc: "Tint the background with the focused item's colours", options: ["On", "Off"] },
        { key: "xy", label: "X and Y buttons", desc: "Swap them if X and Y do each other's job on your controller", options: ["Automatic", "Swapped"] },
        { key: "textsize", label: "Text size", desc: "Make all writing bigger and easier to read", options: ["Normal", "Large", "Largest"] },
        { key: "motion", label: "Motion", desc: "Reduced turns off sliding, zooming, pulsing and moving backgrounds", options: ["Full", "Reduced"] },
        { key: "keyboard", label: "Search keyboard", desc: "Steam's own keyboard opens with search in Gaming Mode", options: ["Arxis", "Steam"] },
        { key: "reset", label: "Reset to defaults", desc: "Put every setting back the way it started", options: [] },
        { key: "about", label: "About & diagnostics", desc: "Version, system and controller info, and a report to share", options: [] }
    ]
    readonly property var settingsDefaults: ({ theme: "Arch Midnight", repeat: "Normal", clock: "24-hour", hints: "Show", glow: "On", xy: "Automatic",
                                               textsize: "Normal", motion: "Full", keyboard: "Arxis" })
    // Read by Rust: exchange X and Y for controllers that report them the other way round.
    readonly property bool xySwap: settings.xy === "Swapped"
    // Accessibility (PROJECT.md §7): every font goes through fs() and every one-off animation
    // through ms(), so Text size scales all writing and Reduced motion makes changes instant.
    // Looping animations (background drift, pulses) are stopped instead.
    readonly property real textScale: settings.textsize === "Largest" ? 1.3 : settings.textsize === "Large" ? 1.15 : 1.0
    readonly property bool reducedMotion: settings.motion === "Reduced"
    function fs(px) { return Math.round(px * textScale) }
    function ms(duration) { return reducedMotion ? 0 : duration }
    // Set by Rust: running in Gaming Mode (gamescope), where Steam's on-screen keyboard exists.
    property bool gamingMode: false
    property int steamKeyboardRequests: 0   // read by the self-test
    property var settings: settingsDefaults
    property int settingsIndex: 0
    property var now: new Date()
    readonly property string themeName: settings.theme
    // Saved by Rust to the settings file whenever it changes (one key=value per line).
    readonly property string settingsBlob: {
        var out = []
        for (var i = 0; i < settingsRows.length; i++)
            if (settingsRows[i].options.length > 0)
                out.push(settingsRows[i].key + "=" + settings[settingsRows[i].key])
        return out.join("\n")
    }
    // Read by Rust: hold-to-repeat "delay,interval" in ms.
    readonly property string repeatTiming: settings.repeat === "Slow" ? "500,160"
                                         : settings.repeat === "Fast" ? "300,70" : "400,110"
    // Set by Rust at start-up from the settings file.
    property string savedSettings: ""
    onSavedSettingsChanged: {
        var next = Object.assign({}, settingsDefaults)
        var lines = savedSettings.split("\n")
        for (var i = 0; i < lines.length; i++) {
            var eq = lines[i].indexOf("=")
            if (eq < 0) continue
            var key = lines[i].slice(0, eq).trim(), value = lines[i].slice(eq + 1).trim()
            var row = settingsRows.find(r => r.key === key)
            if (row && row.options.indexOf(value) >= 0)
                next[key] = value
        }
        settings = next
    }

    function setSetting(key, value) {
        var next = Object.assign({}, settings)
        next[key] = value
        settings = next
    }

    // Moves the focused setting to its previous (-1) or next (+1) option, or runs Reset.
    function changeSetting(dir) {
        var row = settingsRows[settingsIndex]
        if (row.key === "about") {
            edgeBump(dir, 0)
            return
        }
        if (row.key === "reset") {
            settings = Object.assign({}, settingsDefaults)
            showToast("Settings reset to defaults")
            return
        }
        var i = row.options.indexOf(settings[row.key]) + dir
        if (i < 0 || i >= row.options.length) {
            edgeBump(dir, 0)
            return
        }
        setSetting(row.key, row.options[i])
    }

    function handleSettingsAction(name) {
        switch (name) {
        case "up": if (settingsIndex > 0) settingsIndex--; else edgeBump(0, -1); return true
        case "down": if (settingsIndex < settingsRows.length - 1) settingsIndex++; else edgeBump(0, 1); return true
        case "left": changeSetting(-1); return true
        case "right": changeSetting(1); return true
        case "confirm":
            if (settingsRows[settingsIndex].key === "reset") { changeSetting(1); return true }
            if (settingsRows[settingsIndex].key === "about") { openAbout(); return true }
            var row = settingsRows[settingsIndex]
            var i = (row.options.indexOf(settings[row.key]) + 1) % row.options.length
            setSetting(row.key, row.options[i])
            return true
        case "page_up": settingsIndex = 0; return true
        case "page_down": settingsIndex = settingsRows.length - 1; return true
        case "search": case "secondary": case "menu":
            showToast("Not available in Settings")
            return true
        }
        return false
    }

    function itemName(no) { return catalog[no] ? catalog[no].name : "Item " + (no + 1) }
    function itemId(no) { return catalog[no] ? catalog[no].id : "" }
    function itemSummary(no) { return catalog[no] ? catalog[no].summary : "" }
    function itemIcon(no) { return catalog[no] && catalog[no].icon ? catalog[no].icon : "" }
    function categoryOf(tab, no) { return catalog[no] ? catalog[no].category : "" }
    function itemCategory(no) { return categoryOf(tabIndex, no) }
    // Items of the current section, filtered by the search text (name or category)
    // and, when opened from Categories, by one category.
    readonly property var visibleItems: {
        var out = [], q = searchQuery.trim().toLowerCase()
        var n = tabIndex === 0 ? catalog.length : tabCounts[tabIndex]
        for (var i = 0; i < n; i++) {
            if (categoryFilter !== "" && itemCategory(i) !== categoryFilter)
                continue
            if (q === "" || itemName(i).toLowerCase().indexOf(q) >= 0
                    || itemCategory(i).toLowerCase().indexOf(q) >= 0)
                out.push(i)
        }
        return out
    }

    // ---- Categories (third section): tiles that open Discover filtered by one category ----
    readonly property int categoriesTab: 2
    readonly property bool inCategories: root.tabIndex === root.categoriesTab
    readonly property int categoryColumns: 3
    property int categoryIndex: 0
    property string categoryFilter: ""
    function categoryCount(name) {
        var n = 0
        for (var i = 0; i < catalog.length; i++) if (categoryOf(0, i) === name) n++
        return n
    }

    function openCategory(i) {
        switchTab(0)
        categoryFilter = categories[i]
        grid.currentIndex = 0
        grid.positionViewAtIndex(0, GridView.Beginning)
    }

    function handleCategoriesAction(name) {
        var c = categoryIndex, n = categories.length
        switch (name) {
        case "left": if (c % categoryColumns > 0) categoryIndex--; else edgeBump(-1, 0); return true
        case "right": if (c % categoryColumns < categoryColumns - 1 && c + 1 < n) categoryIndex++; else edgeBump(1, 0); return true
        case "up": if (c - categoryColumns >= 0) categoryIndex -= categoryColumns; else edgeBump(0, -1); return true
        case "down": if (c + categoryColumns < n) categoryIndex += categoryColumns; else edgeBump(0, 1); return true
        case "confirm": openCategory(categoryIndex); return true
        case "secondary": case "search": case "menu":
            showToast("Open a category first")
            return true
        }
        return false
    }

    // ---- Installed (second section): apps installed for this user, as reported by Rust ----
    readonly property int installedTab: 1
    readonly property bool inInstalled: root.tabIndex === root.installedTab
    property int installedIndex: 0
    // [{id, name, version}]; follows installedJson, so the list only rebuilds when it changes.
    readonly property var installedItems: installedApps
    readonly property int installedCount: installedItems.length
    onInstalledCountChanged: if (installedIndex >= installedCount) installedIndex = Math.max(0, installedCount - 1)

    function catalogIndexOf(id) {
        for (var i = 0; i < catalog.length; i++) if (catalog[i].id === id) return i
        return -1
    }

    // Queues an update for an installed app ({id, name, icon}); runs through Downloads like an install.
    function updateApp(app) {
        if (queueFindId(app.id) >= 0 && queueModel.get(queueFindId(app.id)).status !== "done") {
            showToast(app.name + " is already in Downloads")
            return false
        }
        var old = queueFindId(app.id)
        if (old >= 0) queueModel.remove(old)
        var no = catalogIndexOf(app.id)
        queueModel.append({ tab: 0, no: no, id: app.id, name: app.name, category: no >= 0 ? categoryOf(0, no) : "",
                            icon: app.icon || "", op: "update", url: "", sum: "", need: 0, file: "", size: 0, version: "", repo: "",
                            error: "", progress: 0, status: "queued" })
        queueRev++
        showToast("Updating " + app.name + (realMode ? "" : " (simulated)"))
        return true
    }

    function updateAll() {
        var n = 0
        for (var i = 0; i < installedItems.length; i++)
            if (hasUpdate(installedItems[i].id) && queueFindId(installedItems[i].id) < 0 && updateApp(installedItems[i])) n++
        if (n > 1) showToast("Updating " + n + " apps" + (realMode ? "" : " (simulated)"))
        else if (n === 0) showToast(updateCount > 0 ? "Updates are already in Downloads" : "Everything is up to date")
    }

    // Confirmation before installing and uninstalling (PROJECT.md §6: ask first, show source,
    // scope, version, size and permissions; ROADMAP.md M2 acceptance).
    property bool confirmOpen: false
    property string confirmKind: "uninstall"   // "install" | "uninstall"
    property int confirmButton: 0           // 0 = Cancel (default), 1 = Install / Uninstall
    property var confirmApp: ({})

    // Details from `flatpak remote-info` per app ID (Rust: details.rs), fetched on demand.
    property var detailsMap: ({})
    function detailsFor(id) { return detailsMap[id] }
    function requestDetails(no) {
        var id = itemId(no)
        if (id && itemSource(no) !== "compat" && detailsMap[id] === undefined) sendCommand("details", id)
    }
    // Real store: fetch the details of the card the user rests on, so the dialog opens ready.
    onCurrentItemNoChanged: if (realMode) detailsPrefetch.restart()
    Timer {
        id: detailsPrefetch
        interval: 600
        onTriggered: if (root.grid_ready()) root.requestDetails(root.currentItemNo)
    }
    function grid_ready() { return grid.count > 0 && tabIndex === 0 }
    readonly property var confirmDetails: confirmKind === "install" ? detailsFor(confirmApp.id || "") : undefined
    // Install is only offered once the details are known (or failed to load, which is shown).
    readonly property bool confirmReady: confirmKind !== "install" || confirmApp.source === "compat" || confirmApp.source === "appimage"
                                         || confirmDetails !== undefined
    readonly property int confirmPermissionCount: confirmDetails && confirmDetails.permissions ? confirmDetails.permissions.length : 0

    // AppImages are saved without the executable bit; making one runnable is a separate,
    // confirmed step (PROJECT.md §5.2 AppImage).
    function askEnable(app) {
        confirmKind = "enable"
        confirmApp = app
        confirmButton = 0
        confirmOpen = true
    }

    function askUninstall(app) {
        confirmKind = "uninstall"
        confirmApp = app
        confirmButton = 0
        confirmOpen = true
    }

    // Y on a card / Install on the details page. In Downloads already: takes it out (no dialog).
    function askInstall(tab, no) {
        if (queueFind(tab, no) >= 0 || isInstalledId(itemId(no))) {
            toggleDownload(tab, no)
            return
        }
        confirmKind = "install"
        confirmApp = { id: itemId(no), name: itemName(no), source: itemSource(no), tab: tab, no: no }
        confirmButton = 0
        requestDetails(no)
        confirmOpen = true
    }

    function handleConfirmAction(name) {
        switch (name) {
        case "left": confirmButton = 0; break
        case "right": confirmButton = 1; break
        case "back": confirmOpen = false; break
        case "confirm":
            if (confirmButton === 1 && !confirmReady) {
                showToast("Still loading the details…")
                return
            }
            confirmOpen = false
            if (confirmButton !== 1) break
            if (confirmKind === "install") toggleDownload(confirmApp.tab, confirmApp.no)
            else if (confirmKind === "enable") sendCommand("register", confirmApp.id)
            else uninstallApp(confirmApp)
            break
        }
    }

    function uninstallApp(it) {
        removingName = it.name
        sendCommand("uninstall", it.id)
        showToast("Uninstalling " + it.name + "…")
    }

    function handleInstalledAction(name) {
        switch (name) {
        case "up": if (installedIndex > 0) installedIndex--; else edgeBump(0, -1); return true
        case "down": if (installedIndex < installedCount - 1) installedIndex++; else edgeBump(0, 1); return true
        case "left": case "right": edgeBump(name === "left" ? -1 : 1, 0); return true
        case "page_up": installedIndex = 0; return true
        case "page_down": installedIndex = Math.max(0, installedCount - 1); return true
        case "confirm":
            if (installedCount === 0) showToast("Finished downloads appear here")
            else if (installedItems[installedIndex].missing === "true") showToast("The file is missing · Y removes the entry")
            else if (installedItems[installedIndex].source === "appimage" && installedItems[installedIndex].registered !== "true")
                askEnable(installedItems[installedIndex])
            else launchApp(installedItems[installedIndex].id)
            return true
        case "secondary":
            if (installedCount > 0) askUninstall(installedItems[installedIndex])
            return true
        case "search":
            if (installedCount === 0) return true
            var app = installedItems[installedIndex]
            if (hasUpdate(app.id)) updateApp(app)
            else showToast(app.name + " is up to date")
            return true
        case "menu":
            updateAll()
            return true
        }
        return false
    }
    readonly property int currentItemNo: grid.currentIndex >= 0 && grid.currentIndex < visibleItems.length
                                         ? visibleItems[grid.currentIndex] : 0

    Theme { id: theme }

    function hueFor(seed) { return (seed * 0.137 + 0.03) % 1.0 }

    // ---- Navigation ----
    function showToast(msg) {
        toast.message = msg
        toast.shown = true
        toast.restartTimer()
    }

    function switchTab(next) {
        if (next < 0 || next >= tabs.length || next === tabIndex) {
            edgeBump(next < 0 ? -1 : 1, 0)
            return
        }
        var f = tabFocus.slice()
        if (searchQuery === "" && categoryFilter === "")
            f[tabIndex] = grid.currentIndex
        tabFocus = f
        searchQuery = ""
        categoryFilter = ""
        slideDir = next > tabIndex ? 1 : -1
        staggerTopRow = Math.max(0, Math.floor(f[next] / columns) - 2)
        staggering = true
        staggerTimer.restart()
        tabIndex = next
        if (next === downloadsTab)
            downloadsIndex = 0
        if (next === installedTab)
            installedIndex = 0
        grid.currentIndex = Math.min(tabFocus[next], grid.count - 1)
        grid.positionViewAtIndex(grid.currentIndex, GridView.Contain)
        sectionIn.restart()
    }

    function moveFocus(to) {
        if (to >= 0 && to < grid.count && to !== grid.currentIndex)
            grid.currentIndex = to
    }

    function openDetail() {
        if (grid.count === 0) {
            showToast("No matches · press B to clear the search")
            return
        }
        requestDetails(currentItemNo)
        var it = grid.currentItem
        if (it) {
            var p = it.mapToItem(stage, it.width / 2, it.height / 2)
            detail.originX = p.x
            detail.originY = p.y
        }
        detailButton = 0
        detail.opened = true
    }

    function openSearch() {
        menuOpen = false
        kbRow = 1
        kbCol = 0
        searchOpen = true
        if (gamingMode && settings.keyboard === "Steam")
            openSteamKeyboard()
    }

    // Steam's on-screen keyboard (Gaming Mode only). Its keys reach the window as normal key
    // presses, which the search already accepts; the Arxis keyboard stays on screen as a fallback.
    function openSteamKeyboard() {
        if (!gamingMode) {
            showToast("Steam's keyboard is only available in Gaming Mode · use the keys on screen")
            return
        }
        sendCommand("steam-keyboard")
    }

    // Closing the keyboard keeps the text; the grid already shows the matches.
    function closeSearch() {
        searchOpen = false
        if (grid.count > 0)
            grid.currentIndex = 0
    }

    function kbPress(value) {
        if (value === "BKSP") searchQuery = searchQuery.slice(0, -1)
        else if (value === "CLEAR") searchQuery = ""
        else if (value === "DONE") closeSearch()
        else if (searchQuery.length >= searchMaxLength) edgeBump(1, 0)
        else if (value === "SPACE") { if (searchQuery.length > 0 && !searchQuery.endsWith(" ")) searchQuery += " " }
        else searchQuery += value
    }

    function kbKeyCenter(row, col) {
        var s = 0
        for (var i = 0; i < col; i++) s += kbRows[row][i][2]
        return s + kbRows[row][col][2] / 2
    }

    function kbColAt(row, center) {
        var s = 0
        for (var i = 0; i < kbRows[row].length; i++) {
            s += kbRows[row][i][2]
            if (center < s) return i
        }
        return kbRows[row].length - 1
    }

    function kbMoveRow(dir) {
        var r = kbRow + dir
        if (r < 0 || r >= kbRows.length) return
        var c = kbColAt(r, kbKeyCenter(kbRow, kbCol))
        kbCol = c
        kbRow = r
    }

    // On-screen keyboard: D-pad moves, A types, Y deletes, X adds a space, View asks for Steam's
    // keyboard, Start or B closes.
    function handleSearchAction(name) {
        switch (name) {
        case "left": if (kbCol > 0) kbCol--; break
        case "right": if (kbCol < kbRows[kbRow].length - 1) kbCol++; break
        case "up": kbMoveRow(-1); break
        case "down": kbMoveRow(1); break
        case "confirm": kbPress(kbRows[kbRow][kbCol][1]); break
        case "secondary": kbPress("BKSP"); break
        case "search": kbPress("SPACE"); break
        case "view": openSteamKeyboard(); break
        case "menu": case "back": closeSearch(); break
        }
    }

    function runMenu(i) {
        menuOpen = false
        if (i === 0) openDetail()
        else if (i === 1) openSearch()
        else if (i === 2) { if (grid.count > 0) askInstall(tabIndex, currentItemNo) }
        else if (i === 3) showToast("Arxis Deck M1 test build · Rust + Qt Quick · controller navigation")
    }

    function handleMenuAction(name) {
        switch (name) {
        case "up": if (menuIndex > 0) menuIndex--; else edgeBump(0, -1); break
        case "down": if (menuIndex < menuItems.length - 1) menuIndex++; else edgeBump(0, 1); break
        case "confirm": runMenu(menuIndex); break
        case "menu": case "back": menuOpen = false; break
        }
    }

    function handleAction(name, source) {
        lastAction = name
        lastSource = source
        if (aboutOpen) {
            handleAboutAction(name)
            return
        }
        if (historyOpen) {
            handleHistoryAction(name)
            return
        }
        if (confirmOpen) {
            handleConfirmAction(name)
            return
        }
        if (searchOpen) {
            handleSearchAction(name)
            return
        }
        if (menuOpen) {
            handleMenuAction(name)
            return
        }
        if (inSettings && !detail.opened && handleSettingsAction(name))
            return
        if (inDownloads && !detail.opened && handleDownloadsAction(name))
            return
        if (inInstalled && !detail.opened && handleInstalledAction(name))
            return
        if (inCategories && !detail.opened && handleCategoriesAction(name))
            return
        if (detail.opened) {
            if (name === "left") detailButton = 0
            else if (name === "right") detailButton = 1
            else if (name === "back") detail.opened = false
            else if (name === "confirm") {
                if (detailButton === 1) detail.opened = false
                else if (queueStatus(tabIndex, currentItemNo) === "" && isInstalledId(itemId(currentItemNo)))
                    launchApp(itemId(currentItemNo))
                else askInstall(tabIndex, currentItemNo)
            }
            return
        }
        var ci = grid.currentIndex
        var row = Math.floor(ci / columns), lastRow = Math.floor((grid.count - 1) / columns)
        switch (name) {
        case "left": if (ci % columns > 0) moveFocus(ci - 1); else edgeBump(-1, 0); break
        case "right":
            if (ci % columns < columns - 1 && ci + 1 < grid.count) moveFocus(ci + 1)
            else edgeBump(1, 0)
            break
        case "up": if (row > 0) moveFocus(ci - columns); else edgeBump(0, -1); break
        case "down": if (row < lastRow) moveFocus(Math.min(ci + columns, grid.count - 1)); else edgeBump(0, 1); break
        case "page_up": moveFocus(Math.max(ci % columns, ci - columns * 3)); break
        case "page_down": if (row < lastRow) moveFocus(Math.min(ci + columns * 3, grid.count - 1)); break
        case "tab_prev": switchTab(tabIndex - 1); break
        case "tab_next": switchTab(tabIndex + 1); break
        case "confirm": openDetail(); break
        case "back":
            if (searchQuery !== "") { searchQuery = ""; grid.currentIndex = 0 }
            else if (categoryFilter !== "") switchTab(categoriesTab)
            else if (tabIndex !== 0) switchTab(0)
            else showToast("Press the Steam button to leave Arxis Deck")
            break
        case "secondary":
            if (grid.count > 0) askInstall(tabIndex, currentItemNo)
            break
        case "search": openSearch(); break
        case "menu": menuIndex = 0; menuOpen = true; break
        case "view": switchTab(downloadsTab); break
        }
    }

    // Keyboard fallback (desktop / docked mode).
    Item {
        id: keys
        focus: true
        Keys.onPressed: (event) => {
            // While searching, a physical keyboard types directly.
            if (root.searchOpen) {
                root.lastAction = "type"
                root.lastSource = "keyboard"
                if (event.key === Qt.Key_Backspace) { root.kbPress("BKSP"); event.accepted = true; return }
                if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Escape) {
                    root.closeSearch(); event.accepted = true; return
                }
                if (event.text.length === 1 && /[a-z0-9 .'\-]/i.test(event.text)) {
                    root.kbPress(event.text === " " ? "SPACE" : event.text.toLowerCase())
                    event.accepted = true
                    return
                }
            }
            var map = {}
            map[Qt.Key_Left] = "left"; map[Qt.Key_Right] = "right"
            map[Qt.Key_Up] = "up"; map[Qt.Key_Down] = "down"
            map[Qt.Key_Return] = "confirm"; map[Qt.Key_Enter] = "confirm"; map[Qt.Key_Space] = "confirm"
            map[Qt.Key_Escape] = "back"; map[Qt.Key_Backspace] = "back"
            map[Qt.Key_Q] = "tab_prev"; map[Qt.Key_E] = "tab_next"
            map[Qt.Key_PageUp] = "page_up"; map[Qt.Key_PageDown] = "page_down"
            map[Qt.Key_X] = "search"; map[Qt.Key_Y] = "secondary"; map[Qt.Key_Slash] = "search"
            map[Qt.Key_M] = "menu"; map[Qt.Key_Menu] = "menu"; map[Qt.Key_Tab] = "view"
            if (map[event.key] !== undefined) {
                root.handleAction(map[event.key], "keyboard")
                event.accepted = true
            }
        }
    }

    Timer {
        id: clockTimer
        interval: 1000; running: true; repeat: true; triggeredOnStart: true
        onTriggered: root.now = new Date()
    }

    // Cards created within this window after a section change (or at start) cascade in.
    Timer { id: staggerTimer; interval: 700; running: true; onTriggered: root.staggering = false }

    // ---- Components ----

    // Capsule art: a gradient with two shapes that drift while the item is active,
    // and a light sweep each time it gains focus (like Steam's capsule glare).

    // A soft round glow made of stacked translucent discs (no blur shader needed).

    // ---- Stage: everything is laid out for 1280x800 and scaled to the window ----
    Item {
        id: stage
        width: 1280
        height: 800
        anchors.centerIn: parent
        scale: Math.min(root.width / 1280, root.height / 800)

        // Ambient background tinted by the focused item, like Big Picture's store pages.
        AmbientBackground { id: ambient }

        // Top bar: brand, L1/R1 section tabs, controller + clock.
        TopBar { id: topBar }

        // Section header
        SectionHeader { id: header }

        // Card grid
        Item {
            id: gridHolder
            property real slide: 0
            visible: !root.inSettings && !root.inDownloads && !root.inInstalled && !root.inCategories
            x: 40 + slide
            y: 172
            width: 1200
            height: 572

            GridView {
                id: grid
                anchors.fill: parent
                cellWidth: 300
                cellHeight: 186
                clip: true
                interactive: false
                cacheBuffer: 400
                model: root.visibleItems
                highlightFollowsCurrentItem: true
                highlightRangeMode: GridView.ApplyRange
                preferredHighlightBegin: 0
                preferredHighlightEnd: height
                highlightMoveDuration: root.ms(180)

                // Focus ring that glides between cards; nudges at the grid edges.
                highlight: Item {
                    id: ring
                    z: 3
                    width: grid.cellWidth
                    height: grid.cellHeight
                    opacity: root.detailOpen || !root.introDone ? 0 : 1
                    Behavior on opacity { NumberAnimation { duration: root.ms(180) } }
                    transform: Translate { id: bumpT }

                    Rectangle {
                        id: ringGlow
                        anchors.centerIn: parent
                        width: 300; height: 182; radius: 22
                        color: "transparent"
                        border.width: 5
                        border.color: theme.alpha(theme.ac, 0.28)
                        SequentialAnimation on opacity {
                            running: !root.reducedMotion
                            loops: Animation.Infinite
                            onStopped: ringGlow.opacity = 1
                            NumberAnimation { from: 1; to: 0.3; duration: 1100; easing.type: Easing.InOutSine }
                            NumberAnimation { from: 0.3; to: 1; duration: 1100; easing.type: Easing.InOutSine }
                        }
                    }
                    Rectangle {
                        anchors.centerIn: parent
                        width: 292; height: 174; radius: 17
                        color: "transparent"
                        border.width: 3
                        border.color: theme.ac
                    }

                    SequentialAnimation {
                        id: bumpAnim
                        property real dx: 0
                        property real dy: 0
                        ParallelAnimation {
                            NumberAnimation { target: bumpT; property: "x"; to: bumpAnim.dx * 9; duration: root.ms(70); easing.type: Easing.OutQuad }
                            NumberAnimation { target: bumpT; property: "y"; to: bumpAnim.dy * 9; duration: root.ms(70); easing.type: Easing.OutQuad }
                        }
                        ParallelAnimation {
                            NumberAnimation { target: bumpT; property: "x"; to: 0; duration: root.ms(260); easing.type: Easing.OutBack; easing.overshoot: 3 }
                            NumberAnimation { target: bumpT; property: "y"; to: 0; duration: root.ms(260); easing.type: Easing.OutBack; easing.overshoot: 3 }
                        }
                    }
                    Connections {
                        target: root
                        function onEdgeBump(dx, dy) {
                            bumpAnim.stop()
                            bumpAnim.dx = dx
                            bumpAnim.dy = dy
                            bumpAnim.start()
                        }
                    }
                }

                delegate: Item {
                    id: cell
                    required property int index
                    required property var modelData
                    readonly property int itemNo: modelData
                    width: grid.cellWidth
                    height: grid.cellHeight
                    z: focused ? 2 : 1
                    readonly property bool focused: GridView.isCurrentItem && !root.detailOpen
                    property real appear: 1

                    Component.onCompleted: {
                        if (!root.staggering)
                            return
                        var row = Math.floor(index / root.columns) - root.staggerTopRow
                        appearDelay.duration = (root.introDone ? 0 : 560)
                                               + Math.max(0, Math.min(3, row)) * 70
                                               + (index % root.columns) * 45
                        appear = 0
                        appearAnim.start()
                    }
                    SequentialAnimation {
                        id: appearAnim
                        PauseAnimation { id: appearDelay; duration: root.ms(0) }
                        NumberAnimation { target: cell; property: "appear"; to: 1; duration: root.ms(380); easing.type: Easing.OutCubic }
                    }

                    // Soft shadow that lifts with the focused card.
                    Rectangle {
                        anchors.centerIn: card
                        anchors.verticalCenterOffset: cell.focused ? 12 : 4
                        width: card.width; height: card.height; radius: 16
                        color: "#000000"
                        opacity: (cell.focused ? 0.5 : 0.2) * cell.appear
                        scale: card.scale
                        Behavior on anchors.verticalCenterOffset { NumberAnimation { duration: root.ms(220); easing.type: Easing.OutCubic } }
                        Behavior on opacity { NumberAnimation { duration: root.ms(220) } }
                    }

                    Rectangle {
                        id: card
                        anchors.centerIn: parent
                        width: 276
                        height: 162
                        radius: 14
                        color: cell.focused ? theme.cardHi : theme.card
                        border.width: 1
                        border.color: theme.bdr
                        scale: (cell.focused ? 1.05 : 1.0) * (0.94 + 0.06 * cell.appear)
                        opacity: cell.appear
                        transform: Translate { y: (1 - cell.appear) * 26 }
                        Behavior on color { ColorAnimation { duration: root.ms(200) } }
                        Behavior on scale { NumberAnimation { duration: root.ms(240); easing.type: Easing.OutBack; easing.overshoot: 2.2 } }

                        Art {
                            x: 1; y: 1
                            width: parent.width - 2; height: 100
                            radius: 0
                            topLeftRadius: 13; topRightRadius: 13
                            seed: cell.itemNo + root.tabIndex * 7
                            icon: root.itemIcon(cell.itemNo)
                            active: cell.focused
                        }
                        // Badge: placeholder, or the item's Downloads status.
                        Rectangle {
                            id: badge
                            // Download status, or "installed" for apps installed earlier.
                            readonly property string status: {
                                var q = root.queueStatus(root.tabIndex, cell.itemNo)
                                return q !== "" ? q : root.isInstalledId(root.itemId(cell.itemNo)) ? "done" : ""
                            }
                            x: 12; y: 12
                            width: chip.implicitWidth + 18; height: 24; radius: 12
                            color: status === "" ? theme.alpha(theme.bg, 0.6)
                                 : status === "done" ? theme.alpha("#16a34a", 0.9)
                                 : status === "failed" ? theme.alpha("#dc2626", 0.9) : theme.alpha(theme.ac, 0.92)
                            Behavior on width { NumberAnimation { duration: root.ms(160) } }
                            Text {
                                id: chip
                                anchors.centerIn: parent
                                text: badge.status === "done" ? (root.hasUpdate(root.itemId(cell.itemNo)) ? "Update available" : "✓ Installed")
                                    : badge.status === "downloading"
                                      ? (root.queueProgress(root.tabIndex, cell.itemNo) > 0
                                         ? "Installing " + Math.round(root.queueProgress(root.tabIndex, cell.itemNo) * 100) + "%" : "Installing…")
                                    : badge.status === "queued" ? "In Downloads"
                                    : badge.status === "failed" ? "Failed"
                                    : badge.status === "paused" ? "Paused"
                                    : root.itemSource(cell.itemNo) === "compat" ? "GitHub"
                                    : root.itemSource(cell.itemNo) === "appimage" ? "AppImage" : root.realMode ? "Flathub" : "Placeholder"
                                color: badge.status === "" || badge.status === "done" || badge.status === "failed" ? theme.tx : theme.bg
                                font.pixelSize: root.fs(12)
                                font.bold: badge.status !== ""
                            }
                        }
                        Column {
                            x: 14; y: 108
                            width: 248
                            spacing: 2
                            Text {
                                width: openHint.visible ? parent.width - openHint.width - 8 : parent.width
                                text: root.itemName(cell.itemNo)
                                color: theme.tx
                                font.pixelSize: root.fs(17)
                                font.bold: true
                                elide: Text.ElideRight
                            }
                            Text {
                                text: root.itemCategory(cell.itemNo)
                                color: cell.focused ? theme.ac : theme.su
                                font.pixelSize: root.fs(14)
                                Behavior on color { ColorAnimation { duration: root.ms(200) } }
                            }
                        }
                        Row {
                            id: openHint
                            anchors.right: parent.right; anchors.rightMargin: 12
                            y: 118
                            spacing: 6
                            property real shift: cell.focused ? 0 : 12
                            opacity: cell.focused ? 1 : 0
                            visible: opacity > 0
                            transform: Translate { x: openHint.shift }
                            Behavior on shift { NumberAnimation { duration: root.ms(220); easing.type: Easing.OutCubic } }
                            Behavior on opacity { NumberAnimation { duration: root.ms(180) } }
                            ButtonGlyph { label: "A"; scale: 0.86 }
                            Text { anchors.verticalCenter: parent.verticalCenter; text: "Open"; color: theme.tx; font.pixelSize: root.fs(15) }
                        }
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: { grid.currentIndex = cell.index; root.handleAction("confirm", "touch") }
                    }
                }
            }

            // Empty search result.
            Column {
                anchors.centerIn: parent
                anchors.verticalCenterOffset: -40
                spacing: 10
                visible: grid.count === 0
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "No matches"
                    color: theme.tx
                    font.pixelSize: root.fs(26)
                    font.bold: true
                }
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: "Press X to search again or B to clear the search."
                    color: theme.su
                    font.pixelSize: root.fs(17)
                }
            }

            // Thin scroll indicator, shown only when the section has more rows than fit.
            Rectangle {
                x: parent.width + 14
                width: 4; height: parent.height; radius: 2
                color: theme.raised
                opacity: grid.visibleArea.heightRatio < 0.999 ? 1 : 0
                Behavior on opacity { NumberAnimation { duration: root.ms(250) } }
                Rectangle {
                    width: parent.width; radius: 2
                    color: theme.ac
                    y: grid.visibleArea.yPosition * parent.height
                    height: grid.visibleArea.heightRatio * parent.height
                }
            }
        }

        // Installed: finished (simulated) downloads. Up/Down picks, A launches, X updates, Y uninstalls (with confirmation).
        LibraryPage { id: installedView }

        // Categories: 3 x 2 tiles. A opens Discover filtered to that category.
        CategoryPage { id: categoriesView }

        // Downloads: one row per item with progress (simulated). Up/Down picks, A pauses/resumes, X removes.
        DownloadsPage { id: downloadsView }

        // Settings: a list on the left (Up/Down picks, Left/Right or A changes), preview on the right.
        SettingsPage { id: settingsView }

        // Section change: header and grid slide in from the direction of travel.
        ParallelAnimation {
            id: sectionIn
            NumberAnimation { target: gridHolder; property: "slide"; from: root.slideDir * 90; to: 0; duration: root.ms(420); easing.type: Easing.OutCubic }
            NumberAnimation { target: header; property: "slide"; from: root.slideDir * 50; to: 0; duration: root.ms(380); easing.type: Easing.OutCubic }
            NumberAnimation { target: header; property: "opacity"; from: 0; to: 1; duration: root.ms(300) }
        }

        // Bottom hint bar: controller status on the left, button hints on the right.
        ControllerHints { id: hintBar }

        // Toast for actions that are not built yet.
        Toast { id: toast }

        // Detail panel (A opens, B closes): zooms out of the focused card into a store page.
        ApplicationPage { id: detail }

        // Search (Y): text field, live preview of matches and a controller keyboard.
        SearchPage { id: searchView }

        // Options menu (Start): the controller's replacement for right-click menus.
        MenuPage { id: menuView }

        // About & diagnostics: version, system, store, controller, files; Y saves a redacted report.
        AboutPage { id: aboutView }

        // Activity history (≡ in Downloads): every install, update, uninstall and launch, newest first.
        HistoryPage { id: historyView }

        // Install / uninstall confirmation: source, scope, version, sizes, permissions. Cancel is focused first.
        ConfirmDialog { id: confirmView }

        // Boot splash: the logo pops in, then the store fades up beneath it.
        Splash { id: splash }
    }
}
