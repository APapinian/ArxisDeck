// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window
import "../components"

Item {
    id: installedView
    visible: root.inInstalled
    x: 52 + gridHolder.slide
    y: 180
    width: 1176
    height: 552

    ListView {
        id: installedList
        anchors.fill: parent
        clip: true
        interactive: false
        spacing: 10
        model: root.installedItems
        currentIndex: root.installedIndex
        highlightFollowsCurrentItem: true
        highlightRangeMode: ListView.ApplyRange
        preferredHighlightBegin: 0
        preferredHighlightEnd: height
        highlightMoveDuration: root.ms(160)
        highlight: Item {}

        delegate: Rectangle {
            id: irow
            required property int index
            required property var modelData
            readonly property bool sel: index === root.installedIndex
            width: installedList.width
            height: 88
            radius: 14
            color: sel ? theme.cardHi : theme.card
            border.width: sel ? 3 : 1
            border.color: sel ? theme.ac : theme.bdr
            Behavior on color { ColorAnimation { duration: root.ms(160) } }

            Art {
                x: 14; y: 12
                width: 64; height: 64; radius: 12
                seed: Math.max(0, root.catalogIndexOf(irow.modelData.id)) + irow.modelData.id.length * 3
                icon: irow.modelData.icon || ""
                active: irow.sel
            }
            Column {
                x: 96
                width: parent.width - 96 - 380
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4
                Text {
                    width: parent.width
                    elide: Text.ElideRight
                    text: irow.modelData.name
                    color: theme.tx
                    font.pixelSize: root.fs(19)
                    font.bold: true
                }
                Text {
                    width: parent.width
                    elide: Text.ElideRight
                    text: irow.modelData.source === "compat" ? "Steam compatibility tool · choose it in a game's Properties"
                        : irow.modelData.source === "appimage"
                          ? "AppImage " + irow.modelData.version + " · " + irow.modelData.path
                            + (irow.modelData.missing === "true" ? " · file missing"
                               : irow.modelData.registered === "true" ? "" : " · not runnable yet: press A")
                        : root.realMode ? irow.modelData.id + (irow.modelData.version ? " · " + irow.modelData.version : "")
                                        : (root.catalogIndexOf(irow.modelData.id) >= 0 ? root.categoryOf(0, root.catalogIndexOf(irow.modelData.id)) + " · " : "") + "sample item"
                    color: theme.su
                    font.pixelSize: root.fs(14)
                }
            }
            Row {
                anchors.right: parent.right
                anchors.rightMargin: 24
                anchors.verticalCenter: parent.verticalCenter
                spacing: 18
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    readonly property string q: (root.queueRev, root.queueFindId(irow.modelData.id) >= 0
                                                 ? queueModel.get(root.queueFindId(irow.modelData.id)).status : "")
                    readonly property bool upd: root.hasUpdate(irow.modelData.id)
                    readonly property bool saved: irow.modelData.source === "appimage" && irow.modelData.registered !== "true"
                    text: q === "downloading" ? "Updating…" : q === "queued" ? "Update waiting"
                        : irow.modelData.missing === "true" ? "⚠ Missing"
                        : saved ? "Saved · not runnable"
                        : upd ? "Update available" : root.realMode ? "✓ Installed" : "✓ Installed (simulated)"
                    color: irow.modelData.missing === "true" ? "#f87171" : saved || upd || q === "downloading" || q === "queued" ? theme.ac : "#4ade80"
                    font.pixelSize: root.fs(16)
                    font.bold: true
                }
                Row {
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 6
                    opacity: irow.sel ? 1 : 0
                    Behavior on opacity { NumberAnimation { duration: root.ms(160) } }
                    ButtonGlyph { label: "Y"; scale: 0.86 }
                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: "Uninstall"
                        color: theme.tx
                        font.pixelSize: root.fs(15)
                    }
                }
            }
            MouseArea {
                anchors.fill: parent
                onClicked: { root.installedIndex = irow.index; root.handleAction("confirm", "touch") }
            }
        }
    }

    Column {
        anchors.centerIn: parent
        anchors.verticalCenterOffset: -40
        spacing: 10
        visible: root.installedCount === 0
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: "Nothing installed yet"
            color: theme.tx
            font.pixelSize: root.fs(26)
            font.bold: true
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.activeCount > 0 ? root.activeCount + " still downloading. Finished downloads appear here."
                                       : "Press Y on any item in Discover to install it."
            color: theme.su
            font.pixelSize: root.fs(17)
        }
    }
}
