// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window
import "../components"

Item {
    id: downloadsView
    visible: root.inDownloads
    x: 52 + gridHolder.slide
    y: 180
    width: 1176
    height: 552

    ListView {
        id: downloadsList
        anchors.fill: parent
        clip: true
        interactive: false
        spacing: 10
        model: queueModel
        currentIndex: root.downloadsIndex
        highlightFollowsCurrentItem: true
        highlightRangeMode: ListView.ApplyRange
        preferredHighlightBegin: 0
        preferredHighlightEnd: height
        highlightMoveDuration: root.ms(160)
        highlight: Item {}

        delegate: Rectangle {
            id: drow
            required property int index
            required property int tab
            required property int no
            required property string name
            required property string category
            required property real progress
            required property string status
            required property string icon
            required property string op
            required property string error
            readonly property bool sel: index === root.downloadsIndex
            width: downloadsList.width
            height: 88
            radius: 14
            color: sel ? theme.cardHi : theme.card
            border.width: sel ? 3 : 1
            border.color: sel ? theme.ac : theme.bdr
            Behavior on color { ColorAnimation { duration: root.ms(160) } }

            Art {
                x: 14; y: 12
                width: 64; height: 64; radius: 12
                seed: drow.no + drow.tab * 7
                icon: drow.icon
                active: drow.status === "downloading"
            }
            Column {
                x: 96
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4
                Text { text: drow.name; color: theme.tx; font.pixelSize: root.fs(19); font.bold: true }
                Text { text: drow.category + " · from " + root.tabs[drow.tab]; color: theme.su; font.pixelSize: root.fs(14) }
            }
            Column {
                x: parent.width - 460
                anchors.verticalCenter: parent.verticalCenter
                width: 420
                spacing: 8
                Item {
                    width: parent.width; height: 22
                    Text {
                        width: parent.width - 80
                        elide: Text.ElideRight
                        text: drow.status === "done" ? (drow.op === "update" ? "✓ Updated" : root.realMode ? "✓ Installed" : "✓ Ready (simulated)")
                            : drow.status === "downloading" ? (drow.op === "update" ? "Updating…" : "Installing…")
                            : drow.status === "failed" ? "Failed: " + (drow.error || "unknown error") + " · A retries"
                            : drow.status === "paused" ? "Paused"
                            : root.networkState === "offline" ? "Waiting for a connection" : "Waiting"
                        color: drow.status === "done" ? "#4ade80" : drow.status === "failed" ? "#f87171"
                             : drow.status === "downloading" ? theme.ac : theme.su
                        font.pixelSize: root.fs(16)
                        font.bold: drow.status === "downloading" || drow.status === "done"
                    }
                    Text {
                        anchors.right: parent.right
                        text: drow.status === "downloading" && drow.progress <= 0 ? "working…"
                              : Math.round(drow.progress * 100) + "%"
                        color: theme.tx
                        font.pixelSize: root.fs(16)
                    }
                }
                Rectangle {
                    id: dbar
                    width: parent.width; height: 8; radius: 4
                    color: theme.glyph
                    clip: true
                    // Working, but no percentage reported yet: a stripe slides across.
                    readonly property bool unknown: drow.status === "downloading" && drow.progress <= 0
                    Rectangle {
                        width: parent.width * drow.progress; height: parent.height; radius: 4
                        visible: !dbar.unknown
                        color: drow.status === "done" ? "#22c55e" : drow.status === "paused" ? theme.su : theme.ac
                        Behavior on width { NumberAnimation { duration: root.ms(100) } }
                    }
                    Rectangle {
                        id: stripe
                        visible: dbar.unknown
                        width: parent.width * 0.28; height: parent.height; radius: 4
                        color: theme.ac
                        NumberAnimation on x {
                            running: dbar.unknown
                            loops: Animation.Infinite
                            from: -stripe.width; to: dbar.width
                            duration: 1400
                            easing.type: Easing.InOutSine
                        }
                    }
                }
            }
            MouseArea {
                anchors.fill: parent
                onClicked: { root.downloadsIndex = drow.index; root.handleAction("confirm", "touch") }
            }
        }
    }

    Column {
        anchors.centerIn: parent
        anchors.verticalCenterOffset: -40
        spacing: 10
        visible: root.queueCount === 0
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Nothing in Downloads yet"
            color: theme.tx
            font.pixelSize: root.fs(26)
            font.bold: true
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Press Y on any item to add it. Downloads here are simulated."
            color: theme.su
            font.pixelSize: root.fs(17)
        }
    }
}
