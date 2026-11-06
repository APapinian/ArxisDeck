// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window

Rectangle {
    id: splash
    anchors.fill: parent
    z: 100
    color: theme.bg
    visible: opacity > 0
    Rectangle {
        id: splashLogo
        anchors.centerIn: parent
        anchors.verticalCenterOffset: -20
        width: 96; height: 96; radius: 26
        color: theme.ac
        scale: 0.5
        opacity: 0
        Rectangle {
            id: splashDiamond
            anchors.centerIn: parent; width: 36; height: 36; color: theme.bg
            rotation: -45
        }
    }
    Text {
        id: splashText
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top: splashLogo.bottom
        anchors.topMargin: 22
        text: "Arxis Deck"
        color: theme.tx
        font.pixelSize: root.fs(28)
        font.bold: true
        opacity: 0
    }
    SequentialAnimation {
        running: true
        ParallelAnimation {
            NumberAnimation { target: splashLogo; property: "scale"; to: 1; duration: root.ms(420); easing.type: Easing.OutBack; easing.overshoot: 2 }
            NumberAnimation { target: splashLogo; property: "opacity"; to: 1; duration: root.ms(220) }
            NumberAnimation { target: splashDiamond; property: "rotation"; to: 45; duration: root.ms(520); easing.type: Easing.OutCubic }
            SequentialAnimation {
                PauseAnimation { duration: root.ms(140) }
                NumberAnimation { target: splashText; property: "opacity"; to: 1; duration: root.ms(260) }
            }
        }
        PauseAnimation { duration: root.ms(120) }
        ScriptAction { script: root.introDone = true }
        ParallelAnimation {
            NumberAnimation { target: splash; property: "opacity"; to: 0; duration: root.ms(320); easing.type: Easing.InOutQuad }
            NumberAnimation { target: splashLogo; property: "scale"; to: 1.25; duration: root.ms(320); easing.type: Easing.InQuad }
        }
    }
}
