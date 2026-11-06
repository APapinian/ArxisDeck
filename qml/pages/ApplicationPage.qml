// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window
import "../components"

Item {
    id: detail
    property bool opened: false
    property real t: 0               // 0 closed … 1 fully open
    property real originX: 640
    property real originY: 400
    anchors.fill: parent
    visible: t > 0.001

    onOpenedChanged: {
        if (opened) { closeAnim.stop(); openAnim.restart() }
        else { openAnim.stop(); closeAnim.restart() }
    }
    NumberAnimation { id: openAnim; target: detail; property: "t"; to: 1; duration: root.ms(380); easing.type: Easing.OutCubic }
    NumberAnimation { id: closeAnim; target: detail; property: "t"; to: 0; duration: root.ms(220); easing.type: Easing.InCubic }

    // Entry stage for staggered content: 0 until t passes k, then 0 → 1.
    function step(k) { return Math.max(0, Math.min(1, (t - k) / (1 - k))) }

    Rectangle {
        anchors.fill: parent
        color: theme.alpha(theme.shade, 0.9)
        opacity: detail.t
    }
    MouseArea { anchors.fill: parent; enabled: detail.opened; onClicked: detail.opened = false }

    Rectangle {
        id: panel
        width: 880
        height: 540
        radius: 20
        color: theme.panel
        border.color: theme.bdr
        x: detail.originX + ((stage.width / 2) - detail.originX) * detail.t - width / 2
        y: detail.originY + ((stage.height / 2 + 12) - detail.originY) * detail.t - height / 2
        scale: 0.32 + 0.68 * detail.t
        opacity: Math.min(1, detail.t * 2.5)
        MouseArea { anchors.fill: parent; enabled: detail.opened }

        Art {
            id: banner
            x: 1; y: 1
            width: parent.width - 2; height: 280
            radius: 0
            topLeftRadius: 19; topRightRadius: 19
            seed: root.currentItemNo + root.tabIndex * 7
            icon: root.itemIcon(root.currentItemNo)
            active: detail.opened
            drift: detail.visible
        }
        Rectangle {
            // fade the banner into the panel
            x: 1; y: banner.y + banner.height - 140
            width: banner.width; height: 140
            gradient: Gradient {
                GradientStop { position: 0.0; color: theme.alpha(theme.panel, 0) }
                GradientStop { position: 1.0; color: theme.panel }
            }
        }

        Column {
            x: 40; y: 196
            width: 520
            spacing: 8
            opacity: detail.step(0.25)
            transform: Translate { y: (1 - detail.step(0.25)) * 22 }
            Text {
                text: root.itemName(root.currentItemNo)
                color: theme.tx
                font.pixelSize: root.fs(36)
                font.bold: true
            }
            Text {
                text: root.itemCategory(root.currentItemNo) + "  ·  " + root.tabs[root.tabIndex]
                color: theme.ac
                font.pixelSize: root.fs(17)
                font.bold: true
            }
        }

        Text {
            x: 40; y: 316
            width: 500
            // Ends with "…" above the buttons, whatever the text size.
            height: panel.height - 40 - 56 - 14 - y
            wrapMode: Text.WordWrap
            text: root.itemSource(root.currentItemNo) === "appimage"
                  ? root.itemSummary(root.currentItemNo)
                    + "\nSaved to ~/Applications and not run automatically: you choose afterwards whether to make it runnable. "
                    + "The SHA-256 comes from GitHub: it catches broken downloads, not who published it."
                  : root.itemSource(root.currentItemNo) === "compat"
                  ? root.itemSummary(root.currentItemNo)
                    + "\nNever replaces a version you already have. The checksum comes from the same release: "
                    + "it catches broken downloads, not who published it. Afterwards restart Steam and pick it in a game's Properties → Compatibility."
                  : root.realMode
                  ? (root.itemSummary(root.currentItemNo) || "No description.")
                    + "\n\nInstalled from Flathub for this user only, so no password is needed and SteamOS updates keep it."
                  : "This is a placeholder details page for the controller test. Real descriptions, screenshots, sizes and install options come in a later milestone."
            elide: Text.ElideRight
            color: theme.su
            font.pixelSize: root.fs(17)
            lineHeight: 1.25
            opacity: detail.step(0.45)
            transform: Translate { y: (1 - detail.step(0.45)) * 18 }
        }

        // Info column, like the store page's side facts.
        Column {
            x: 590; y: 312
            width: 250
            spacing: 10
            opacity: detail.step(0.55)
            transform: Translate { x: (1 - detail.step(0.55)) * 24 }
            Repeater {
                model: {
                    var q = root.queueStatus(root.tabIndex, root.currentItemNo)
                    var installed = root.isInstalledId(root.itemId(root.currentItemNo))
                    var src = root.itemSource(root.currentItemNo)
                    var compat = src === "compat" || src === "appimage"
                    var d = root.detailsFor(root.itemId(root.currentItemNo))
                    var info = function (k) { return d === undefined ? "…" : d.error ? "unknown" : (d[k] || "—") }
                    var status = ["Status", installed ? "Installed" : q === "" ? "Not installed"
                                            : q.charAt(0).toUpperCase() + q.slice(1)]
                    if (compat)
                        return [["Source", src === "appimage" ? "GitHub · AppImage" : "GitHub releases"],
                                ["Integrity", !root.catalog[root.currentItemNo].sum ? "not verified"
                                              : src === "appimage" ? "SHA-256 (from GitHub)" : "SHA-512 (same release)"],
                                status]
                    return [["Source", root.realMode ? "Flathub (user)" : "Simulated"],
                            ["Version", info("version")],
                            ["Download", info("download")],
                            ["Permissions", d === undefined ? "…" : d.error ? "unknown"
                                            : d.permissions.length + (d.attention > 0 ? " · " + d.attention + " ⚠" : "")],
                            status]
                }
                Item {
                    required property var modelData
                    width: 250; height: 30
                    Rectangle { anchors.bottom: parent.bottom; width: parent.width; height: 1; color: theme.bdr }
                    Text { text: parent.modelData[0]; color: theme.su; font.pixelSize: root.fs(15) }
                    Text {
                        anchors.right: parent.right
                        width: 170
                        horizontalAlignment: Text.AlignRight
                        elide: Text.ElideLeft
                        text: parent.modelData[1]
                        color: theme.tx
                        font.pixelSize: root.fs(15)
                    }
                }
            }
        }

        Item {
            x: 40
            y: panel.height - 40 - 56
            width: 220 * 2 + 16
            height: 56
            opacity: detail.step(0.6)
            transform: Translate { y: (1 - detail.step(0.6)) * 18 }

            // Selection plate that slides between the two buttons.
            Rectangle {
                id: btnSel
                width: 220; height: 56; radius: 12
                x: root.detailButton * (220 + 16)
                color: theme.ac
                Behavior on x { NumberAnimation { duration: root.ms(220); easing.type: Easing.OutCubic } }
                Rectangle {
                    id: btnPulse
                    anchors.centerIn: parent
                    width: parent.width + 10; height: parent.height + 10; radius: 16
                    color: "transparent"
                    border.width: 3; border.color: theme.alpha(theme.ac, 0.3)
                    SequentialAnimation on opacity {
                        running: detail.visible && !root.reducedMotion; loops: Animation.Infinite
                        onStopped: btnPulse.opacity = 1
                        NumberAnimation { from: 1; to: 0.25; duration: 900; easing.type: Easing.InOutSine }
                        NumberAnimation { from: 0.25; to: 1; duration: 900; easing.type: Easing.InOutSine }
                    }
                }
            }
            Row {
                spacing: 16
                Repeater {
                    model: [root.queueStatus(root.tabIndex, root.currentItemNo) !== "" ? "Remove download"
                            : root.isInstalledId(root.itemId(root.currentItemNo)) ? "Launch"
                            : root.realMode ? "Install" : "Add to Downloads", "Back"]
                    Rectangle {
                        required property int index
                        required property string modelData
                        readonly property bool sel: index === root.detailButton
                        width: 220; height: 56; radius: 12
                        color: sel ? "transparent" : theme.raised
                        border.width: sel ? 0 : 1
                        border.color: theme.bdr
                        Text {
                            anchors.centerIn: parent
                            text: parent.modelData
                            color: parent.sel ? theme.bg : theme.tx
                            font.pixelSize: root.fs(18)
                            font.bold: true
                            Behavior on color { ColorAnimation { duration: root.ms(180) } }
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: { root.detailButton = parent.index; root.handleAction("confirm", "touch") }
                        }
                    }
                }
            }
        }
    }
}
