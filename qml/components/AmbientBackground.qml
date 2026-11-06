// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window

Item {
    id: ambient
    anchors.fill: parent
    opacity: root.settings.glow === "On" ? 1 : 0
    visible: opacity > 0
    Behavior on opacity { NumberAnimation { duration: root.ms(300) } }
    property color tint: Qt.hsla(root.focusHue, 0.7, 0.45, 1)
    Behavior on tint { ColorAnimation { duration: root.ms(650); easing.type: Easing.OutCubic } }

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: Qt.rgba(ambient.tint.r, ambient.tint.g, ambient.tint.b, 0.30) }
            GradientStop { position: 0.45; color: Qt.rgba(ambient.tint.r, ambient.tint.g, ambient.tint.b, 0.06) }
            GradientStop { position: 1.0; color: "transparent" }
        }
    }
    Orb {
        id: orbA
        width: 900; height: 900
        x: 520; y: -520
        tint: ambient.tint
        strength: 0.022
        SequentialAnimation on x {
            running: !root.reducedMotion
            loops: Animation.Infinite
            NumberAnimation { to: 260; duration: 16000; easing.type: Easing.InOutSine }
            NumberAnimation { to: 620; duration: 16000; easing.type: Easing.InOutSine }
        }
    }
    Orb {
        width: 700; height: 700
        x: -260; y: 380
        tint: Qt.hsla((root.focusHue + 0.5) % 1.0, 0.6, 0.45, 1)
        strength: 0.012
        SequentialAnimation on y {
            running: !root.reducedMotion
            loops: Animation.Infinite
            NumberAnimation { to: 300; duration: 13000; easing.type: Easing.InOutSine }
            NumberAnimation { to: 420; duration: 13000; easing.type: Easing.InOutSine }
        }
    }
}
