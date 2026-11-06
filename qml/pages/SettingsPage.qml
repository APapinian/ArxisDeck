// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window
import "../components"

Item {
    id: settingsView
    visible: root.inSettings
    x: 52 + gridHolder.slide
    y: 180
    width: 1176
    height: 552

    // The list is taller than the page; it scrolls to keep the focused row in view.
    // (The clip is a little wider than the rows so the focused row's zoom is not cut.)
    Item {
        id: settingsClip
        x: -8
        width: 796
        height: parent.height
        clip: true

        Item {
            id: settingsList
            x: 8
            width: 780
            height: root.settingsRows.length * 68 - 6
            y: -Math.max(0, Math.min((root.settingsIndex - 2) * 68, height - settingsClip.height))
            Behavior on y { NumberAnimation { duration: root.ms(200); easing.type: Easing.OutCubic } }

            Repeater {
                model: root.settingsRows
                Rectangle {
                    id: srow
                    required property int index
                    required property var modelData
                    readonly property bool sel: index === root.settingsIndex && !root.menuOpen && !root.searchOpen
                    readonly property var opts: modelData.options
                    readonly property int pos: opts.indexOf(root.settings[modelData.key])
                    y: index * 68
                    width: settingsList.width
                    height: 62
                    radius: 14
                    color: sel ? theme.cardHi : theme.card
                    border.width: sel ? 3 : 1
                    border.color: sel ? theme.ac : theme.bdr
                    scale: sel ? 1.015 : 1.0
                    Behavior on scale { NumberAnimation { duration: root.ms(180); easing.type: Easing.OutCubic } }
                    Behavior on color { ColorAnimation { duration: root.ms(160) } }

                    Column {
                        x: 24
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 3
                        // Leaves room for the value on the right.
                        Text { width: srow.width - 300; elide: Text.ElideRight; text: srow.modelData.label; color: theme.tx; font.pixelSize: root.fs(19); font.bold: true }
                        Text { width: srow.width - 300; elide: Text.ElideRight; text: srow.modelData.desc; color: theme.su; font.pixelSize: root.fs(14) }
                    }

                    // Value with arrows (options) or a button (reset).
                    Row {
                        anchors.right: parent.right
                        anchors.rightMargin: 20
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 14
                        visible: srow.opts.length > 0
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "◀"
                            color: srow.sel ? theme.ac : theme.su
                            opacity: srow.pos > 0 ? 1 : 0.25
                            font.pixelSize: root.fs(16)
                            MouseArea { anchors.fill: parent; anchors.margins: -12; onClicked: { root.settingsIndex = srow.index; root.handleAction("left", "touch") } }
                        }
                        Column {
                            anchors.verticalCenter: parent.verticalCenter
                            spacing: 6
                            width: 190
                            Text {
                                width: parent.width
                                horizontalAlignment: Text.AlignHCenter
                                text: root.settings[srow.modelData.key] || ""
                                color: theme.tx
                                font.pixelSize: root.fs(18)
                                font.bold: srow.sel
                            }
                            Row {
                                anchors.horizontalCenter: parent.horizontalCenter
                                spacing: 5
                                Repeater {
                                    model: srow.opts.length
                                    Rectangle {
                                        required property int index
                                        width: index === srow.pos ? 16 : 6; height: 6; radius: 3
                                        color: index === srow.pos ? theme.ac : theme.glyph
                                        Behavior on width { NumberAnimation { duration: root.ms(160) } }
                                    }
                                }
                            }
                        }
                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "▶"
                            color: srow.sel ? theme.ac : theme.su
                            opacity: srow.pos < srow.opts.length - 1 ? 1 : 0.25
                            font.pixelSize: root.fs(16)
                            MouseArea { anchors.fill: parent; anchors.margins: -12; onClicked: { root.settingsIndex = srow.index; root.handleAction("right", "touch") } }
                        }
                    }
                    Row {
                        anchors.right: parent.right
                        anchors.rightMargin: 24
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 8
                        visible: srow.opts.length === 0
                        ButtonGlyph { label: "A"; opacity: srow.sel ? 1 : 0.5 }
                        Text { anchors.verticalCenter: parent.verticalCenter; text: srow.modelData.key === "about" ? "Open" : "Reset"; color: theme.tx; font.pixelSize: root.fs(17) }
                    }
                    MouseArea {
                        anchors.fill: parent
                        z: -1
                        onClicked: { root.settingsIndex = srow.index; root.handleAction("confirm", "touch") }
                    }
                }
            }
        }
    }

    // Live preview of the theme and options.
    Rectangle {
        x: settingsList.width + 28
        width: parent.width - x
        height: parent.height
        radius: 18
        color: theme.panel
        border.color: theme.bdr
        Behavior on color { ColorAnimation { duration: root.ms(300) } }

        Column {
            x: 24; y: 22
            width: parent.width - 48
            spacing: 14
            Text { text: "Preview"; color: theme.su; font.pixelSize: root.fs(15); font.bold: true }
            Rectangle {
                width: parent.width; height: 196; radius: 14
                color: theme.cardHi
                border.width: 3
                border.color: theme.ac
                Art {
                    x: 3; y: 3
                    width: parent.width - 6; height: 120
                    radius: 0
                    topLeftRadius: 11; topRightRadius: 11
                    seed: 3
                    active: root.inSettings
                }
                Text { x: 16; y: 134; text: "Sample item 4"; color: theme.tx; font.pixelSize: root.fs(18); font.bold: true }
                Text { x: 16; y: 160; text: "Utility"; color: theme.ac; font.pixelSize: root.fs(15) }
                Row {
                    anchors.right: parent.right; anchors.rightMargin: 14
                    y: 146
                    spacing: 6
                    ButtonGlyph { label: "A"; scale: 0.86 }
                    Text { anchors.verticalCenter: parent.verticalCenter; text: "Open"; color: theme.tx; font.pixelSize: root.fs(15) }
                }
            }
            Text { text: root.themeName; color: theme.tx; font.pixelSize: root.fs(20); font.bold: true }
            // One swatch per theme; the chosen one is ringed.
            Flow {
                width: parent.width
                spacing: 10
                Repeater {
                    model: root.settingsRows[0].options
                    Rectangle {
                        required property string modelData
                        readonly property var d: root.themeDefs[modelData]
                        width: 30; height: 30; radius: 15
                        color: d[0]
                        border.width: modelData === root.themeName ? 3 : 1
                        border.color: modelData === root.themeName ? theme.tx : theme.bdr
                        Rectangle { anchors.centerIn: parent; width: 14; height: 14; radius: 7; color: parent.d[1] }
                        MouseArea { anchors.fill: parent; onClicked: root.setSetting("theme", parent.modelData) }
                    }
                }
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: "Hold-to-repeat: " + root.repeatTiming.split(",")[0] + " ms before repeating, then every "
                      + root.repeatTiming.split(",")[1] + " ms."
                color: theme.su
                font.pixelSize: root.fs(14)
            }
            Text {
                width: parent.width
                wrapMode: Text.WordWrap
                text: "Text " + Math.round(root.textScale * 100) + "% · "
                      + (root.reducedMotion ? "changes are instant" : "full motion")
                color: theme.su
                font.pixelSize: root.fs(14)
            }
        }
    }
}
