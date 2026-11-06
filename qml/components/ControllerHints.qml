// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window

Rectangle {
    id: hintBar
    property alias hintCount: hintRepeater.count
    property alias statusLine: statusText.text
    z: 50
    y: parent.height - height
    width: parent.width
    height: 48
    color: theme.alpha(theme.panel, 0.94)
    Rectangle { width: parent.width; height: 1; color: theme.bdr }

    Text {
        id: statusText
        x: 28
        anchors.verticalCenter: parent.verticalCenter
        width: Math.max(120, hintRow.x - x - 24)
        elide: Text.ElideRight
        text: root.controllerStatus + (root.lastAction !== "" ? "   ·   Last input: " + root.lastAction + " (" + root.lastSource + ")" : "")
        color: theme.su
        font.pixelSize: root.fs(15)
    }

    Row {
        id: hintRow
        visible: root.settings.hints === "Show"
        anchors.right: parent.right
        anchors.rightMargin: 28
        anchors.verticalCenter: parent.verticalCenter
        spacing: 20
        Repeater {
            id: hintRepeater
            model: root.aboutOpen
                   ? [["▲▼", "Scroll"], ["Y", "Save report"], ["B", "Close"]]
                   : root.historyOpen
                   ? [["▲▼", "Scroll"], ["B", "Close"]]
                   : root.confirmOpen
                   ? [["◀▶", "Choose"], ["A", "Select"], ["B", "Cancel"]]
                   : root.searchOpen
                   ? (root.gamingMode ? [["A", "Type"], ["Y", "Delete"], ["X", "Space"], ["⧉", "Steam keyboard"], ["≡", "Done"], ["B", "Close"]]
                                     : [["A", "Type"], ["Y", "Delete"], ["X", "Space"], ["≡", "Done"], ["B", "Close"]])
                   : root.menuOpen
                   ? [["▲▼", "Choose"], ["A", "Select"], ["B", "Close"]]
                   : root.detailOpen
                   ? [["◀▶", "Choose"], ["A", "Select"], ["B", "Close"]]
                   : root.inSettings
                   ? [["▲▼", "Choose"], ["◀▶", "Change"], ["B", "Back"], ["L1", "R1"]]
                   : root.inInstalled
                   ? [["A", "Launch"], ["X", "Update"], ["Y", "Uninstall"], ["≡", "Update all"], ["B", "Back"]]
                   : root.inCategories
                   ? [["A", "Open"], ["B", "Back"], ["L1", "R1"]]
                   : root.categoryFilter !== "" && root.searchQuery === ""
                   ? [["A", "Open"], ["B", "Categories"], ["Y", "Download"], ["X", "Search"], ["≡", "Options"]]
                   : root.inDownloads
                   ? [["▲▼", "Choose"], ["A", root.selectedDownloadStatus === "done" ? "Launch" : "Pause / Resume"], ["Y", "Remove"], ["≡", "History"], ["B", "Back"]]
                   : root.searchQuery !== ""
                   ? [["A", "Open"], ["B", "Clear search"], ["X", "Search"], ["≡", "Options"]]
                   : [["A", "Open"], ["B", "Back"], ["Y", "Download"], ["X", "Search"], ["≡", "Options"], ["L1", "R1"]]
            Row {
                id: hint
                required property int index
                required property var modelData
                spacing: 8
                opacity: 0
                transform: Translate { id: hintT; y: 10 }
                Component.onCompleted: hintIn.start()
                SequentialAnimation {
                    id: hintIn
                    PauseAnimation { duration: root.ms(hint.index * 35) }
                    ParallelAnimation {
                        NumberAnimation { target: hint; property: "opacity"; to: 1; duration: root.ms(200) }
                        NumberAnimation { target: hintT; property: "y"; to: 0; duration: root.ms(260); easing.type: Easing.OutCubic }
                    }
                }
                ButtonGlyph { label: hint.modelData[0] }
                ButtonGlyph { label: hint.modelData[1]; visible: hint.modelData[0] === "L1" }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: hint.modelData[0] === "L1" ? "Sections" : hint.modelData[1]
                    color: theme.tx
                    font.pixelSize: root.fs(16)
                }
            }
        }
    }
}
