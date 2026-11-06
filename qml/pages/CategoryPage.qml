// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window
import "../components"

Item {
    id: categoriesView
    visible: root.inCategories
    x: 52 + gridHolder.slide
    y: 180
    width: 1176
    height: 552

    Repeater {
        model: root.categories
        Item {
            id: ctile
            required property int index
            required property string modelData
            readonly property bool sel: index === root.categoryIndex
            readonly property int col: index % root.categoryColumns
            readonly property int row: Math.floor(index / root.categoryColumns)
            x: col * (376 + 24)
            y: row * (258 + 16)
            width: 376
            height: 258
            z: sel ? 2 : 1

            Rectangle {
                id: tileCard
                anchors.fill: parent
                radius: 18
                color: theme.card
                border.width: ctile.sel ? 3 : 1
                border.color: ctile.sel ? theme.ac : theme.bdr
                scale: ctile.sel ? 1.03 : 1.0
                Behavior on scale { NumberAnimation { duration: root.ms(220); easing.type: Easing.OutBack; easing.overshoot: 2 } }

                Art {
                    x: 2; y: 2
                    width: parent.width - 4; height: 166
                    radius: 0
                    topLeftRadius: 16; topRightRadius: 16
                    seed: ctile.index * 5 + 2
                    active: ctile.sel
                }
                Text {
                    x: 22; y: 180
                    text: ctile.modelData
                    color: theme.tx
                    font.pixelSize: root.fs(26)
                    font.bold: true
                }
                Text {
                    x: 22; y: 218
                    text: root.categoryCount(ctile.modelData) + " items in Discover"
                    color: ctile.sel ? theme.ac : theme.su
                    font.pixelSize: root.fs(15)
                }
                Row {
                    anchors.right: parent.right; anchors.rightMargin: 18
                    y: 208
                    spacing: 6
                    opacity: ctile.sel ? 1 : 0
                    Behavior on opacity { NumberAnimation { duration: root.ms(160) } }
                    ButtonGlyph { label: "A" }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: "Open"; color: theme.tx; font.pixelSize: root.fs(16) }
                }
            }
            MouseArea {
                anchors.fill: parent
                onClicked: { root.categoryIndex = ctile.index; root.handleAction("confirm", "touch") }
            }
        }
    }
}
