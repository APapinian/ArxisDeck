#!/bin/sh
# SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
# SPDX-License-Identifier: GPL-3.0-or-later
# Builds Arxis Deck as a Flatpak and writes a single-file bundle, arxis-deck.flatpak,
# that can be copied to a Steam Deck and installed there (see packaging/README.md).
#
# First run downloads the build tools for this user from Flathub (about 1.5 GB):
# org.flatpak.Builder, org.kde.Sdk 6.11 and the Rust SDK extension 25.08.
set -eu

ID=io.github.fatih_bucaklioglu.ArxisDeck
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user -y --noninteractive flathub \
    org.flatpak.Builder \
    org.kde.Platform//6.11 \
    org.kde.Sdk//6.11 \
    org.freedesktop.Sdk.Extension.rust-stable//25.08

flatpak run org.flatpak.Builder --user --force-clean --disable-rofiles-fuse \
    --repo=build/flatpak-repo build/flatpak-build "packaging/flatpak/$ID.yml"

flatpak build-bundle --runtime-repo=https://dl.flathub.org/repo/flathub.flatpakrepo \
    build/flatpak-repo arxis-deck.flatpak "$ID"

echo
echo "Done: $ROOT/arxis-deck.flatpak"
