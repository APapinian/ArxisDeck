// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window
import "../components"

Item {
    id: historyView
    anchors.fill: parent
    z: 44
    property real t: root.historyOpen ? 1 : 0
    Behavior on t { NumberAnimation { duration: root.ms(200); easing.type: Easing.OutCubic } }
    visible: t > 0.001

    Rectangle { anchors.fill: parent; color: theme.alpha(theme.shade, 0.88); opacity: historyView.t }
    MouseArea { anchors.fill: parent; enabled: root.historyOpen; onClicked: root.historyOpen = false }

    Rectangle {
        x: 120; y: 96
        width: parent.width - 240
        height: parent.height - 96 - 48 - 28
        radius: 20
        color: theme.panel
        border.color: theme.bdr
        opacity: historyView.t
        scale: 0.96 + 0.04 * historyView.t
        MouseArea { anchors.fill: parent; enabled: root.historyOpen }

        Text {
            x: 32; y: 24
            text: "Activity history"
            color: theme.tx
            font.pixelSize: root.fs(28)
            font.bold: true
        }
        Text {
            x: 32; y: 64
            text: root.historyCount + (root.historyCount === 1 ? " operation" : " operations")
                  + " · newest first · kept on this device in arxis-deck.sqlite"
            color: theme.su
            font.pixelSize: root.fs(15)
        }

        ListView {
            id: historyList
            x: 24; y: 100
            width: parent.width - 48
            height: parent.height - 100 - 20
            clip: true
            interactive: false
            spacing: 6
            model: root.historyOps
            currentIndex: root.historyIndex
            highlightFollowsCurrentItem: true
            highlightRangeMode: ListView.ApplyRange
            preferredHighlightBegin: 0
            preferredHighlightEnd: height
            highlightMoveDuration: root.ms(120)
            highlight: Item {}

            delegate: Rectangle {
                id: hrow
                required property int index
                required property var modelData
                readonly property bool sel: index === root.historyIndex
                readonly property bool ok: modelData.result === "ok"
                width: historyList.width
                height: 52
                radius: 10
                color: sel ? theme.cardHi : theme.card
                border.width: sel ? 2 : 0
                border.color: theme.ac

                Text {
                    x: 16
                    anchors.verticalCenter: parent.verticalCenter
                    width: 130
                    text: Qt.formatDateTime(new Date(hrow.modelData.started_at), "dd MMM  HH:mm")
                    color: theme.su
                    font.pixelSize: root.fs(15)
                }
                Text {
                    x: 150
                    anchors.verticalCenter: parent.verticalCenter
                    width: 120
                    text: hrow.modelData.result ? root.historyVerb(hrow.modelData.kind)
                          : ({ install: "Installing", update: "Updating", uninstall: "Uninstalling" })[hrow.modelData.kind] || hrow.modelData.kind
                    color: theme.tx
                    font.pixelSize: root.fs(16)
                    font.bold: true
                }
                Text {
                    x: 275
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - 275 - 340
                    elide: Text.ElideRight
                    text: root.appName(hrow.modelData.target_id)
                    color: theme.tx
                    font.pixelSize: root.fs(16)
                }
                Text {
                    anchors.right: resultText.left
                    anchors.rightMargin: 16
                    anchors.verticalCenter: parent.verticalCenter
                    text: hrow.modelData.source
                    color: theme.su
                    font.pixelSize: root.fs(14)
                }
                Text {
                    id: resultText
                    anchors.right: parent.right
                    anchors.rightMargin: 16
                    anchors.verticalCenter: parent.verticalCenter
                    width: 190
                    horizontalAlignment: Text.AlignRight
                    elide: Text.ElideRight
                    // Text, not only colour, tells the outcome (PROJECT.md §7).
                    text: hrow.ok ? "✓ done" : hrow.modelData.result ? "✗ " + hrow.modelData.result : "running…"
                    color: hrow.ok ? "#4ade80" : hrow.modelData.result ? "#f87171" : theme.ac
                    font.pixelSize: root.fs(15)
                    font.bold: true
                }
            }
        }

        Text {
            anchors.centerIn: parent
            visible: root.historyCount === 0
            text: "Nothing yet. Installs, updates, uninstalls and launches appear here."
            color: theme.su
            font.pixelSize: root.fs(17)
        }
    }
}
