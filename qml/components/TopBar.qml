// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window

Rectangle {
    id: topBar
    property alias tabCount: tabRepeater.count
    width: parent.width
    height: 72
    color: theme.alpha(theme.panel, 0.88)
    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: theme.bdr }

    Row {
        x: 28
        anchors.verticalCenter: parent.verticalCenter
        spacing: 12
        Rectangle {
            width: 36; height: 36; radius: 10
            color: theme.ac
            Rectangle {
                anchors.centerIn: parent; width: 14; height: 14; color: theme.bg
                rotation: 45 + root.tabIndex * 90
                Behavior on rotation { NumberAnimation { duration: root.ms(420); easing.type: Easing.OutBack } }
            }
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: "Arxis Deck"
            color: theme.tx
            font.pixelSize: root.fs(22)
            font.bold: true
        }
    }

    Row {
        id: tabRow
        anchors.centerIn: parent
        anchors.horizontalCenterOffset: 40
        spacing: 6
        ButtonGlyph {
            id: l1Glyph
            label: "L1"; anchors.verticalCenter: parent.verticalCenter
            opacity: root.tabIndex > 0 ? 1 : 0.35
            Behavior on opacity { NumberAnimation { duration: root.ms(200) } }
        }
        Repeater {
            id: tabRepeater
            model: root.tabs
            Item {
                required property int index
                required property string modelData
                readonly property bool current: index === root.tabIndex
                width: tabLabel.implicitWidth + 32
                height: 72
                Text {
                    id: tabLabel
                    anchors.centerIn: parent
                    text: parent.modelData
                    color: parent.current ? theme.tx : theme.su
                    font.pixelSize: root.fs(19)
                    font.bold: parent.current
                    scale: parent.current ? 1.0 : 0.96
                    Behavior on color { ColorAnimation { duration: root.ms(220) } }
                    Behavior on scale { NumberAnimation { duration: root.ms(220); easing.type: Easing.OutCubic } }
                }
                // Number of available updates, on the Installed tab.
                Rectangle {
                    visible: parent.index === root.installedTab && root.updateCount > 0
                    anchors.left: tabLabel.right
                    anchors.leftMargin: 2
                    anchors.bottom: tabLabel.verticalCenter
                    width: Math.max(18, updateBadgeText.implicitWidth + 8); height: 18; radius: 9
                    color: theme.ac
                    Text {
                        id: updateBadgeText
                        anchors.centerIn: parent
                        text: root.updateCount
                        color: theme.bg
                        font.pixelSize: root.fs(11)
                        font.bold: true
                    }
                }
                MouseArea { anchors.fill: parent; onClicked: root.switchTab(parent.index) }
            }
        }
        ButtonGlyph {
            id: r1Glyph
            label: "R1"; anchors.verticalCenter: parent.verticalCenter
            opacity: root.tabIndex < root.tabs.length - 1 ? 1 : 0.35
            Behavior on opacity { NumberAnimation { duration: root.ms(200) } }
        }
    }

    // One underline that slides to the selected tab.
    Rectangle {
        id: tabLine
        readonly property Item target: tabRepeater.count > root.tabIndex ? tabRepeater.itemAt(root.tabIndex) : null
        y: parent.height - height
        height: 4; radius: 2
        color: theme.ac
        x: target ? tabRow.x + target.x + 8 : 0
        width: target ? target.width - 16 : 0
        Behavior on x { enabled: root.introDone; NumberAnimation { duration: root.ms(320); easing.type: Easing.OutCubic } }
        Behavior on width { enabled: root.introDone; NumberAnimation { duration: root.ms(320); easing.type: Easing.OutCubic } }
    }

    Row {
        anchors.right: parent.right
        anchors.rightMargin: 28
        anchors.verticalCenter: parent.verticalCenter
        spacing: 16
        // Offline: a word and a symbol, not colour alone (PROJECT.md §7).
        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.networkState === "offline"
            width: offlineText.implicitWidth + 20; height: 28; radius: 14
            color: theme.alpha("#dc2626", 0.25)
            border.color: "#f87171"
            Text {
                id: offlineText
                anchors.centerIn: parent
                text: "⚠ Offline"
                color: "#fca5a5"
                font.pixelSize: root.fs(15)
                font.bold: true
            }
        }
        // Download indicator: count of active downloads and the current one's progress.
        Row {
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8
            visible: root.activeCount > 0
            Text {
                anchors.verticalCenter: parent.verticalCenter
                text: "↓ " + root.activeCount
                color: theme.ac
                font.pixelSize: root.fs(17)
                font.bold: true
            }
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: 56; height: 6; radius: 3
                color: theme.glyph
                Rectangle {
                    width: parent.width * root.activeProgress; height: parent.height; radius: 3
                    color: theme.ac
                }
            }
        }
        Item {
            anchors.verticalCenter: parent.verticalCenter
            width: 12; height: 12
            readonly property bool on: root.controllerConnected === "true"
            Rectangle {
                // pulse ring while a controller is connected
                id: padPulse
                anchors.centerIn: parent
                width: 12; height: 12; radius: width / 2
                color: "transparent"
                border.color: "#22c55e"; border.width: 2
                opacity: 0
                visible: parent.on
                ParallelAnimation {
                    running: padPulse.visible && !root.reducedMotion; loops: Animation.Infinite
                    onStopped: padPulse.opacity = 0
                    NumberAnimation { target: padPulse; property: "width"; from: 12; to: 30; duration: 1600; easing.type: Easing.OutCubic }
                    NumberAnimation { target: padPulse; property: "height"; from: 12; to: 30; duration: 1600; easing.type: Easing.OutCubic }
                    NumberAnimation { target: padPulse; property: "opacity"; from: 0.7; to: 0; duration: 1600 }
                }
            }
            Rectangle {
                anchors.fill: parent; radius: 6
                color: parent.on ? "#22c55e" : "#64748b"
                Behavior on color { ColorAnimation { duration: root.ms(300) } }
            }
        }
        Text {
            id: clock
            text: Qt.formatTime(root.now, root.settings.clock === "12-hour" ? "h:mm AP" : "HH:mm")
            color: theme.tx
            font.pixelSize: root.fs(19)
        }
    }
}
