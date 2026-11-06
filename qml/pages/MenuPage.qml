// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window
import "../components"

Item {
    id: menuView
    anchors.fill: parent
    z: 40
    property real t: root.menuOpen ? 1 : 0
    Behavior on t { NumberAnimation { duration: root.ms(240); easing.type: Easing.OutCubic } }
    visible: t > 0.001

    Rectangle { anchors.fill: parent; color: theme.alpha(theme.shade, 0.63); opacity: menuView.t }
    MouseArea { anchors.fill: parent; enabled: root.menuOpen; onClicked: root.menuOpen = false }

    Rectangle {
        id: sheet
        width: 400
        y: 72
        height: parent.height - 72 - 48
        x: parent.width - width * menuView.t
        color: theme.panel
        border.color: theme.bdr
        MouseArea { anchors.fill: parent; enabled: root.menuOpen }

        Column {
            x: 28; y: 28
            width: sheet.width - 56
            spacing: 6
            Text { text: "Options"; color: theme.tx; font.pixelSize: root.fs(26); font.bold: true }
            Text {
                width: parent.width
                elide: Text.ElideRight
                text: grid.count > 0 ? root.itemName(root.currentItemNo) + " · " + root.itemCategory(root.currentItemNo) : "No item selected"
                color: theme.su
                font.pixelSize: root.fs(16)
            }
            Item { width: 1; height: 18 }
            Item {
                width: parent.width
                height: root.menuItems.length * 62

                // Selection plate that slides between entries.
                Rectangle {
                    width: parent.width; height: 56; radius: 12
                    y: root.menuIndex * 62
                    color: theme.ac
                    Behavior on y { NumberAnimation { duration: root.ms(180); easing.type: Easing.OutCubic } }
                }
                Repeater {
                    model: root.menuItems
                    Item {
                        required property int index
                        required property string modelData
                        y: index * 62
                        width: parent.width; height: 56
                        Text {
                            x: 20
                            anchors.verticalCenter: parent.verticalCenter
                            text: parent.modelData
                            color: parent.index === root.menuIndex ? theme.bg : theme.tx
                            font.pixelSize: root.fs(19)
                            font.bold: parent.index === root.menuIndex
                            Behavior on color { ColorAnimation { duration: root.ms(150) } }
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: { root.menuIndex = parent.index; root.handleAction("confirm", "touch") }
                        }
                    }
                }
            }
        }
    }
}
