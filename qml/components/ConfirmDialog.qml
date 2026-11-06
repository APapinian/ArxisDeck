// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Arxis Deck interface, used by Main.qml. Names like `root`, `theme` and
// the other parts' ids come from Main.qml.
import QtQuick
import QtQuick.Window

Item {
    id: confirmView
    anchors.fill: parent
    z: 45
    property real t: root.confirmOpen ? 1 : 0
    Behavior on t { NumberAnimation { duration: root.ms(200); easing.type: Easing.OutCubic } }
    visible: t > 0.001
    readonly property bool compat: root.confirmApp.source === "compat" || root.confirmApp.source === "appimage"
    readonly property bool appimage: root.confirmApp.source === "appimage"

    Rectangle { anchors.fill: parent; color: theme.alpha(theme.shade, 0.85); opacity: confirmView.t }
    MouseArea { anchors.fill: parent; enabled: root.confirmOpen; onClicked: root.confirmOpen = false }

    Rectangle {
        anchors.centerIn: parent
        width: 760
        height: root.confirmKind !== "install" ? 330 : confirmView.compat ? 400 : 600
        radius: 20
        color: theme.panel
        border.color: theme.bdr
        scale: 0.94 + 0.06 * confirmView.t
        opacity: confirmView.t
        MouseArea { anchors.fill: parent; enabled: root.confirmOpen }

        Column {
            x: 36; y: 32
            width: parent.width - 72
            spacing: 12
            Text {
                width: parent.width
                elide: Text.ElideRight
                text: (root.confirmKind === "install" ? "Install " : root.confirmKind === "enable" ? "Make runnable: " : "Uninstall ")
                      + (root.confirmApp.name || "") + "?"
                color: theme.tx
                font.pixelSize: root.fs(28)
                font.bold: true
            }
            // Uninstall: what goes and what stays.
            Text {
                visible: root.confirmKind !== "install"
                width: parent.width
                wrapMode: Text.WordWrap
                lineHeight: 1.25
                color: theme.su
                font.pixelSize: root.fs(16)
                text: confirmView.compat
                      ? "Source: Steam compatibility tool you installed\nRemoves the folder " + (root.confirmApp.name || "")
                        + " from Steam's compatibility tools. Games set to use it fall back to Steam's default Proton."
                      : root.confirmKind === "enable"
                      ? "Marks " + (root.confirmApp.path || "the file") + " as a program and adds it to the app menu, so Steam's "
                        + "\"Add a Non-Steam Game\" can find it.\nIt runs with your user's rights, outside any sandbox. Only do this for software you trust."
                      : confirmView.appimage
                      ? "Source: AppImage from GitHub\nTakes away only the files Arxis Deck created: " + (root.confirmApp.path || "the AppImage")
                        + " and its app-menu entry. Its settings in your home folder stay."
                      : "Source: Flatpak · " + (root.realMode ? "user installation" : "simulated") + " · " + (root.confirmApp.id || "")
                        + "\nThe app is removed for your user. Its settings and saved data in ~/.var/app stay."
            }
            // Install: source and scope, then version and sizes (or integrity for GitHub downloads).
            Text {
                visible: root.confirmKind === "install"
                width: parent.width
                wrapMode: Text.WordWrap
                lineHeight: 1.25
                color: theme.su
                font.pixelSize: root.fs(16)
                text: confirmView.appimage
                      ? "From github.com/" + ((root.catalog[root.confirmApp.no] || {}).repo || "") + " → ~/Applications · not run automatically"
                      : confirmView.compat
                      ? "From GitHub releases → Steam's compatibility tools folder · no password"
                      : "From " + (root.realMode ? "Flathub" : "the simulated store") + " · installed for your user only, no password · "
                        + (root.confirmApp.id || "")
            }
            Text {
                visible: root.confirmKind === "install"
                width: parent.width
                wrapMode: Text.WordWrap
                lineHeight: 1.25
                color: theme.tx
                font.pixelSize: root.fs(17)
                readonly property var d: root.confirmDetails
                readonly property var item: root.catalog[root.confirmApp.no] || ({})
                text: confirmView.compat
                      ? (item.summary || "") + "\nIntegrity: "
                        + (!item.sum ? "not verified — this release publishes no checksum"
                           : confirmView.appimage ? "SHA-256 from GitHub, plus size and program checks (catches broken downloads, does not prove the publisher)"
                           : "SHA-512 from the same release (catches broken downloads, does not prove the publisher)")
                        + (confirmView.appimage ? "\nAfterwards you decide whether to make it runnable." : "")
                      : d === undefined ? "Loading version, size and permissions…"
                      : d.error ? "Details unavailable: " + d.error
                      : "Version " + (d.version || "—") + "  ·  download " + (d.download || "—")
                        + "  ·  installed " + (d.installed || "—") + "\nLicence " + (d.license || "—")
                        + (d.runtime ? "  ·  needs " + d.runtime.split("/")[0] : "")
            }
            // Permissions, most important first; level shown by a symbol and a word, not colour alone.
            Column {
                visible: root.confirmKind === "install" && !confirmView.compat && root.confirmPermissionCount > 0
                width: parent.width
                spacing: 4
                Text {
                    text: "Permissions (" + root.confirmPermissionCount + ")"
                          + (root.confirmDetails && root.confirmDetails.attention > 0
                             ? " · " + root.confirmDetails.attention + " need attention" : "")
                    color: theme.tx
                    font.pixelSize: root.fs(17)
                    font.bold: true
                }
                Repeater {
                    model: root.confirmDetails && root.confirmDetails.permissions ? root.confirmDetails.permissions.slice(0, 7) : []
                    Text {
                        required property var modelData
                        width: parent.width
                        elide: Text.ElideRight
                        text: (modelData.level === "high" ? "⚠ attention: " : modelData.level === "medium" ? "•  " : "·  ")
                              + modelData.label
                        color: modelData.level === "high" ? "#f87171" : modelData.level === "medium" ? theme.tx : theme.su
                        font.pixelSize: root.fs(15)
                    }
                }
                Text {
                    visible: root.confirmPermissionCount > 7
                    text: "+ " + (root.confirmPermissionCount - 7) + " more"
                    color: theme.su
                    font.pixelSize: root.fs(15)
                }
            }
        }

        Row {
            x: 36
            y: parent.height - 36 - 56
            spacing: 16
            Repeater {
                model: ["Cancel", root.confirmKind === "install" ? (root.confirmReady ? "Install" : "Loading…")
                                  : root.confirmKind === "enable" ? "Make runnable" : "Uninstall"]
                Rectangle {
                    required property int index
                    required property string modelData
                    readonly property bool sel: index === root.confirmButton
                    readonly property bool danger: index === 1 && root.confirmKind === "uninstall"
                    width: 220; height: 56; radius: 12
                    opacity: index === 1 && !root.confirmReady ? 0.5 : 1
                    color: sel ? (danger ? "#dc2626" : theme.ac) : theme.raised
                    border.width: sel ? 0 : 1
                    border.color: theme.bdr
                    Behavior on color { ColorAnimation { duration: root.ms(150) } }
                    Text {
                        anchors.centerIn: parent
                        text: parent.modelData
                        color: parent.sel ? (parent.danger ? "white" : theme.bg) : theme.tx
                        font.pixelSize: root.fs(18)
                        font.bold: true
                    }
                    MouseArea {
                        anchors.fill: parent
                        onClicked: { root.confirmButton = parent.index; root.handleAction("confirm", "touch") }
                    }
                }
            }
        }
    }
}
