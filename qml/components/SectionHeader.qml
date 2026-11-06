// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window

Item {
    id: header
    property alias subtitle: headerSub.text
    property real slide: 0
    x: 52 + slide
    y: 92
    width: 1176
    height: 70
    Column {
        spacing: 4
        Text {
            text: root.searchQuery !== "" ? "Results for “" + root.searchQuery.trim() + "”"
                  : root.categoryFilter !== "" ? root.categoryFilter : root.tabs[root.tabIndex]
            color: theme.tx
            font.pixelSize: root.fs(30)
            font.bold: true
        }
        Text {
            id: headerSub
            text: root.networkState === "offline" && (root.tabIndex === 0 || root.inDownloads) && root.searchQuery === ""
                  ? "⚠ Offline · " + root.networkMessage
                  : root.searchQuery !== ""
                  ? grid.count + (grid.count === 1 ? " matching item" : " matching items") + " in " + root.tabs[root.tabIndex]
                    + (root.searchStatusText !== "" ? " · " + root.searchStatusText : "") + " · B clears the search"
                  : root.inSettings
                  ? "Changes apply right away and are saved on this device."
                  : root.categoryFilter !== ""
                  ? grid.count + " items in this category · B returns to Categories"
                  : root.inInstalled
                  ? (root.realMode ? "Flatpak apps installed for this user. " : "Finished downloads (simulated). ")
                    + (root.updateCount > 0 ? root.updateCount + (root.updateCount === 1 ? " update" : " updates")
                                              + " available · X updates one, ≡ updates all"
                                            : "Everything is up to date.")
                  : root.inCategories
                  ? "Pick a category to see its items in Discover."
                  : root.inDownloads
                  ? (root.realMode ? "Installs from Flathub for this user, no password needed. "
                                   : "Simulated: nothing is installed in this test build. ")
                    + root.activeCount + " active, " + root.doneCount + " done."
                  : root.realMode
                  ? "Flathub apps picked for Steam Deck · X searches all of Flathub · " + root.backendStatus
                  : root.backendStatus !== "" ? root.backendStatus
                  : "Placeholder items for the controller test. Store features arrive in later milestones."
            color: root.networkState === "offline" && (root.tabIndex === 0 || root.inDownloads) && root.searchQuery === ""
                   ? "#f87171" : root.searchQuery !== "" || root.categoryFilter !== "" ? theme.ac : theme.su
            font.pixelSize: root.fs(16)
        }
    }
    Text {
        anchors.right: parent.right
        y: 10
        text: root.inSettings ? (root.settingsIndex + 1) + " / " + root.settingsRows.length
              : root.inDownloads ? (root.queueCount > 0 ? (root.downloadsIndex + 1) + " / " + root.queueCount : "0 / 0")
              : root.inInstalled ? (root.installedCount > 0 ? (root.installedIndex + 1) + " / " + root.installedCount : "0 / 0")
              : root.inCategories ? (root.categoryIndex + 1) + " / " + root.categories.length
              : grid.count > 0 ? (grid.currentIndex + 1) + " / " + grid.count : "0 / 0"
        color: theme.su
        font.pixelSize: root.fs(16)
    }
}
