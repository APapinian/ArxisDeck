// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window

Rectangle {
    property string label
    height: 28
    width: label.length > 1 ? 40 : 28
    radius: label.length > 1 ? 8 : 14
    color: theme.glyph
    border.color: theme.glyphBdr
    Text {
        anchors.centerIn: parent
        text: parent.label
        color: "white"
        font.pixelSize: 14
        font.bold: true
    }
}
