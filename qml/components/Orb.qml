// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window

Item {
    id: orb
    property color tint: "white"
    property real strength: 0.05
    Repeater {
        model: 14
        Rectangle {
            required property int index
            anchors.centerIn: parent
            width: orb.width * (1 - index * 0.065); height: width; radius: width / 2
            color: Qt.rgba(orb.tint.r, orb.tint.g, orb.tint.b, orb.strength)
        }
    }
}
