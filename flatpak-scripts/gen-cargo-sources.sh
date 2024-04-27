#!/usr/bin/env bash

set -e

SYS_PYTHON3="$(which python3)"
SCRIPTS_DIR="flatpak-scripts"


if [ ! -d "${SCRIPTS_DIR}/.venv" ]; then
  $SYS_PYTHON3 -m venv "${SCRIPTS_DIR}/.venv"
fi

source "${SCRIPTS_DIR}/.venv/bin/activate"
pip install -r "${SCRIPTS_DIR}/requirements.txt"

# Generate Cargo sources file
CARGO_SRCS="cargo-sources.json"
python "${SCRIPTS_DIR}/flatpak-cargo-generator.py" Cargo.lock -o $CARGO_SRCS

# Convert to YAML
python "${SCRIPTS_DIR}/flatpak-json2yaml.py" -f $CARGO_SRCS
rm $CARGO_SRCS
