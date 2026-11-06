// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window
import "../components"

Item {
    id: aboutView
    property alias flick: aboutFlick
    anchors.fill: parent
    z: 44
    property real t: root.aboutOpen ? 1 : 0
    Behavior on t { NumberAnimation { duration: root.ms(200); easing.type: Easing.OutCubic } }
    visible: t > 0.001

    Rectangle { anchors.fill: parent; color: theme.alpha(theme.shade, 0.88); opacity: aboutView.t }
    MouseArea { anchors.fill: parent; enabled: root.aboutOpen; onClicked: root.aboutOpen = false }

    Rectangle {
        x: 120; y: 96
        width: parent.width - 240
        height: parent.height - 96 - 48 - 28
        radius: 20
        color: theme.panel
        border.color: theme.bdr
        opacity: aboutView.t
        scale: 0.96 + 0.04 * aboutView.t
        MouseArea { anchors.fill: parent; enabled: root.aboutOpen }

        Row {
            x: 32; y: 24
            spacing: 14
            Rectangle {
                width: 40; height: 40; radius: 11
                color: theme.ac
                Rectangle { anchors.centerIn: parent; width: 15; height: 15; rotation: 45; color: theme.bg }
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: "About & diagnostics"
                color: theme.tx
                font.pixelSize: root.fs(28)
                font.bold: true
            }
        }
        Text {
            x: 32; y: 74
            width: parent.width - 64
            elide: Text.ElideRight
            text: root.exportPath !== "" ? "Last report: " + root.exportPath
                                         : "Y saves this page and recent log lines as a text file to share; your home folder is shown as ~"
            color: root.exportPath !== "" ? theme.ac : theme.su
            font.pixelSize: root.fs(15)
        }

        Flickable {
            id: aboutFlick
            x: 32; y: 108
            width: parent.width - 64
            height: parent.height - 108 - 20
            clip: true
            contentHeight: aboutColumn.height
            Behavior on contentY { NumberAnimation { duration: root.ms(140) } }

            Column {
                id: aboutColumn
                width: aboutFlick.width
                spacing: 14
                Repeater {
                    model: root.aboutSections
                    Column {
                        required property var modelData
                        width: aboutColumn.width
                        spacing: 4
                        Text {
                            text: parent.modelData.title
                            color: theme.ac
                            font.pixelSize: root.fs(18)
                            font.bold: true
                        }
                        Repeater {
                            model: parent.modelData.rows
                            Row {
                                required property var modelData
                                spacing: 16
                                Text {
                                    width: 220
                                    text: parent.modelData.key
                                    color: theme.su
                                    font.pixelSize: root.fs(15)
                                }
                                Text {
                                    width: aboutColumn.width - 236
                                    wrapMode: Text.WrapAnywhere
                                    text: parent.modelData.value
                                    color: theme.tx
                                    font.pixelSize: root.fs(15)
                                }
                            }
                        }
                    }
                }
                Text {
                    visible: root.aboutSections.length === 0
                    text: "Collecting…"
                    color: theme.su
                    font.pixelSize: root.fs(16)
                }
            }
        }
    }
}
