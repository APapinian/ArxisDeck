// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window

Rectangle {
    id: toast
    property alias message: toastText.text
    function restartTimer() { toastTimer.restart() }
    z: 60
    property bool shown: false
    anchors.horizontalCenter: parent.horizontalCenter
    y: hintBar.y - height - 20 + (shown ? 0 : 26)
    width: toastText.implicitWidth + 64
    height: 48
    radius: 24
    color: theme.raised
    border.color: theme.ac
    opacity: shown ? 1 : 0
    scale: shown ? 1 : 0.92
    Behavior on opacity { NumberAnimation { duration: root.ms(180) } }
    Behavior on y { NumberAnimation { duration: root.ms(280); easing.type: Easing.OutCubic } }
    Behavior on scale { NumberAnimation { duration: root.ms(280); easing.type: Easing.OutBack } }
    Rectangle {
        x: 18; anchors.verticalCenter: parent.verticalCenter
        width: 10; height: 10; radius: 5; color: theme.ac
    }
    Text { id: toastText; x: 40; anchors.verticalCenter: parent.verticalCenter; color: theme.tx; font.pixelSize: root.fs(17) }
    Timer { id: toastTimer; interval: 2200; onTriggered: toast.shown = false }
}
