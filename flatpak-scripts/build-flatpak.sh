#!/usr/bin/env bash

set -e

APP_ID="dev.bdavidson.BiosRenamer"
BUILD_DIR="flatpak-build"
SCRIPTS_DIR="flatpak-scripts"

"${SCRIPTS_DIR}/gen-cargo-sources.sh"
flatpak-builder --force-clean $BUILD_DIR "${APP_ID}.yml"