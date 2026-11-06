// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window

Rectangle {
    id: art
    property int seed: 0
    property bool active: false
    property bool drift: false     // slow continuous float (details banner)
    property string icon: ""       // file:// URL of the app's real icon, shown over the gradient
    readonly property bool hasIcon: icon !== "" && iconImage.status === Image.Ready
    readonly property real hue: root.hueFor(seed)
    clip: true
    gradient: Gradient {
        orientation: Gradient.Horizontal
        GradientStop { position: 0.0; color: Qt.hsla(art.hue, 0.62, 0.50, 1) }
        GradientStop { position: 1.0; color: Qt.hsla((art.hue + 0.09) % 1.0, 0.55, 0.24, 1) }
    }

    Rectangle {
        id: bigShape
        readonly property real s: Math.min(art.width, art.height) * 0.6
        width: s; height: s
        x: art.width * 0.64 - s / 2 + (art.active ? -art.width * 0.05 : 0) + floatX
        y: (art.height - s) / 2 + floatY
        property real floatX: 0
        property real floatY: 0
        radius: art.seed % 3 === 0 ? s / 2 : (art.seed % 3 === 1 ? s * 0.14 : 0)
        rotation: (art.seed % 3 === 2 ? 45 : 0) + (art.active ? 10 : 0)
        color: "#ffffff"
        opacity: art.hasIcon ? 0.08 : art.active ? 0.3 : 0.22
        Behavior on x { NumberAnimation { duration: root.ms(520); easing.type: Easing.OutCubic } }
        Behavior on rotation { NumberAnimation { duration: root.ms(620); easing.type: Easing.OutCubic } }
        Behavior on opacity { NumberAnimation { duration: root.ms(300) } }
        SequentialAnimation on floatY {
            running: art.drift && !root.reducedMotion; loops: Animation.Infinite
            NumberAnimation { to: -art.height * 0.06; duration: 2600; easing.type: Easing.InOutSine }
            NumberAnimation { to: art.height * 0.06; duration: 2600; easing.type: Easing.InOutSine }
        }
        SequentialAnimation on floatX {
            running: art.drift && !root.reducedMotion; loops: Animation.Infinite
            NumberAnimation { to: -art.width * 0.03; duration: 3700; easing.type: Easing.InOutSine }
            NumberAnimation { to: art.width * 0.03; duration: 3700; easing.type: Easing.InOutSine }
        }
    }
    Rectangle {
        readonly property real s: Math.min(art.width, art.height) * 0.26
        width: s; height: s; radius: s / 2
        x: art.width * 0.2 - s / 2 + (art.active ? art.width * 0.06 : 0) - bigShape.floatX * 1.6
        y: art.height * 0.62 - s / 2 + (art.active ? -art.height * 0.08 : 0) - bigShape.floatY * 1.6
        color: "#ffffff"
        opacity: 0.14
        Behavior on x { NumberAnimation { duration: root.ms(700); easing.type: Easing.OutCubic } }
        Behavior on y { NumberAnimation { duration: root.ms(700); easing.type: Easing.OutCubic } }
    }

    // The app's own icon, centred; lifts a little when focused.
    Image {
        id: iconImage
        readonly property real s: Math.min(art.width, art.height) * 0.72
        source: art.icon
        visible: art.hasIcon
        asynchronous: true
        smooth: true
        mipmap: true
        fillMode: Image.PreserveAspectFit
        sourceSize.width: 128
        sourceSize.height: 128
        width: s; height: s
        x: (art.width - s) / 2 + bigShape.floatX * 0.5
        y: (art.height - s) / 2 + bigShape.floatY * 0.5 - (art.active ? art.height * 0.03 : 0)
        scale: art.active ? 1.06 : 1.0
        Behavior on scale { NumberAnimation { duration: root.ms(260); easing.type: Easing.OutBack } }
        Behavior on y { NumberAnimation { duration: root.ms(260); easing.type: Easing.OutCubic } }
    }

    Rectangle {
        id: sweep
        width: art.width * 0.35
        height: art.height * 2.2
        y: -art.height * 0.6
        x: -width * 2
        rotation: 20
        opacity: 0.0
        gradient: Gradient {
            orientation: Gradient.Horizontal
            GradientStop { position: 0.0; color: "#00ffffff" }
            GradientStop { position: 0.5; color: "#55ffffff" }
            GradientStop { position: 1.0; color: "#00ffffff" }
        }
    }
    ParallelAnimation {
        id: sweepAnim
        NumberAnimation { target: sweep; property: "x"; from: -sweep.width * 1.5; to: art.width + sweep.width * 0.5; duration: root.ms(720); easing.type: Easing.InOutQuad }
        SequentialAnimation {
            NumberAnimation { target: sweep; property: "opacity"; to: 1; duration: root.ms(140) }
            PauseAnimation { duration: root.ms(420) }
            NumberAnimation { target: sweep; property: "opacity"; to: 0; duration: root.ms(160) }
        }
    }
    onActiveChanged: if (active) sweepAnim.restart()
}
