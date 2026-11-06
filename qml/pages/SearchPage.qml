// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window
import "../components"

Item {
    id: searchView
    anchors.fill: parent
    z: 30
    property real t: root.searchOpen ? 1 : 0
    Behavior on t { NumberAnimation { duration: root.ms(260); easing.type: Easing.OutCubic } }
    visible: t > 0.001

    Rectangle { anchors.fill: parent; color: theme.alpha(theme.shade, 0.94); opacity: searchView.t }
    MouseArea { anchors.fill: parent; enabled: root.searchOpen }

    Rectangle {
        id: field
        x: 140
        y: 92 - (1 - searchView.t) * 30
        width: 1000
        height: 64
        radius: 14
        color: theme.panel
        border.width: 2
        border.color: theme.ac
        opacity: searchView.t

        // Magnifier glyph
        Item {
            x: 22; anchors.verticalCenter: parent.verticalCenter
            width: 26; height: 26
            Rectangle { width: 18; height: 18; radius: 9; color: "transparent"; border.width: 3; border.color: theme.su }
            Rectangle { x: 14; y: 17; width: 11; height: 4; radius: 2; rotation: 45; color: theme.su }
        }
        Text {
            id: queryText
            x: 64
            anchors.verticalCenter: parent.verticalCenter
            text: root.searchQuery !== "" ? root.searchQuery : "Search " + root.tabs[root.tabIndex] + "…"
            color: root.searchQuery !== "" ? theme.tx : theme.su
            font.pixelSize: root.fs(24)
        }
        Rectangle {
            id: caret
            x: root.searchQuery !== "" ? queryText.x + queryText.implicitWidth + 3 : queryText.x - 6
            anchors.verticalCenter: parent.verticalCenter
            width: 2; height: 30
            color: theme.ac
            SequentialAnimation on opacity {
                running: searchView.visible; loops: Animation.Infinite
                NumberAnimation { to: 0; duration: 450; easing.type: Easing.InQuad }
                NumberAnimation { to: 1; duration: 450; easing.type: Easing.OutQuad }
            }
        }
        Text {
            anchors.right: parent.right; anchors.rightMargin: 22
            anchors.verticalCenter: parent.verticalCenter
            text: root.searchQuery.length + " / " + root.searchMaxLength
            color: theme.su
            font.pixelSize: root.fs(15)
        }
    }

    Text {
        x: 140; y: 176
        opacity: searchView.t
        text: root.searchQuery === ""
              ? "Type to filter " + root.tabs[root.tabIndex] + " by name or category"
              : grid.count + (grid.count === 1 ? " match" : " matches") + " in " + root.tabs[root.tabIndex]
                + (root.searchStatusText !== "" ? " · " + root.searchStatusText : "")
        color: root.searchQuery !== "" && grid.count === 0 ? "#f87171" : theme.su
        font.pixelSize: root.fs(17)
    }

    // First matches, so the result is visible while typing.
    Row {
        x: 140; y: 214
        spacing: 16
        opacity: searchView.t
        Repeater {
            model: root.searchQuery !== "" ? root.visibleItems.slice(0, 4) : []
            Rectangle {
                required property var modelData
                width: 238; height: 76; radius: 12
                color: theme.card
                border.color: theme.bdr
                Art {
                    x: 10; y: 10; width: 56; height: 56; radius: 10
                    seed: parent.modelData + root.tabIndex * 7
                    icon: root.itemIcon(parent.modelData)
                }
                Column {
                    x: 78; anchors.verticalCenter: parent.verticalCenter
                    width: 150
                    spacing: 2
                    Text { width: parent.width; elide: Text.ElideRight; text: root.itemName(parent.parent.modelData); color: theme.tx; font.pixelSize: root.fs(16); font.bold: true }
                    Text { text: root.itemCategory(parent.parent.modelData); color: theme.su; font.pixelSize: root.fs(14) }
                }
            }
        }
    }

    // Keyboard: 10 units per row, unit pitch 84 px.
    Column {
        id: keyboard
        readonly property int unit: 84
        readonly property int gap: 8
        x: (parent.width - (10 * unit - gap)) / 2
        y: 330 + (1 - searchView.t) * 60
        spacing: keyboard.gap
        opacity: searchView.t
        Repeater {
            model: root.kbRows.length
            Row {
                id: kbRowItem
                required property int index
                spacing: keyboard.gap
                Repeater {
                    model: root.kbRows[kbRowItem.index]
                    Rectangle {
                        id: key
                        required property int index
                        required property var modelData
                        readonly property bool sel: root.kbRow === kbRowItem.index && root.kbCol === index
                        readonly property bool special: modelData[1].length > 1
                        width: modelData[2] * keyboard.unit - keyboard.gap
                        height: 64
                        radius: 10
                        color: sel ? theme.ac : (special ? theme.raised : theme.card)
                        border.width: sel ? 0 : 1
                        border.color: theme.bdr
                        scale: sel ? 1.07 : 1.0
                        z: sel ? 2 : 1
                        Behavior on scale { NumberAnimation { duration: root.ms(140); easing.type: Easing.OutBack; easing.overshoot: 2 } }
                        Behavior on color { ColorAnimation { duration: root.ms(120) } }
                        Text {
                            anchors.centerIn: parent
                            text: key.modelData[0]
                            color: key.sel ? theme.bg : theme.tx
                            font.pixelSize: root.fs(key.special ? 18 : 24)
                            font.bold: key.sel || key.special
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: {
                                root.kbRow = kbRowItem.index
                                root.kbCol = key.index
                                root.handleAction("confirm", "touch")
                            }
                        }
                    }
                }
            }
        }
    }
}
