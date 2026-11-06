// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick

QtObject {
    id: theme
    // Background and accent come from the chosen Arxis theme (Settings → Theme);
    // panels, cards and borders are the background lifted toward a cool slate.
    readonly property var def: root.themeDefs[root.themeName] || root.themeDefs["Arch Midnight"]
    readonly property color bg: def[0]
    readonly property color ac: def[1]
    readonly property color slate: "#8fa6d6"
    readonly property color panel: lift(0.055)
    readonly property color card: lift(0.075)
    readonly property color cardHi: lift(0.11)
    readonly property color raised: lift(0.12)
    readonly property color bdr: lift(0.14)
    readonly property color glyph: lift(0.2)
    readonly property color glyphBdr: lift(0.3)
    readonly property color shade: Qt.darker(bg, 1.8)
    readonly property color tx: "#e8edf6"
    readonly property color su: "#8b97ad"
    function lift(a) { return Qt.tint(bg, Qt.rgba(slate.r, slate.g, slate.b, a)) }
    function alpha(c, a) { return Qt.rgba(c.r, c.g, c.b, a) }
}
