#!/usr/bin/env bash
# /************************************************
# * File: install.sh
# * Author: Michal Švrček
# *
# * NetProbe Linux installer.
# *
# * ver. 0.6.0
# *************************************************/

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

SOURCE="${1:-$PROJECT_ROOT/target/release/nprobe}"

if [[ ! -f "$SOURCE" ]]; then
    SOURCE="$SCRIPT_DIR/nprobe"
fi

if [[ ! -f "$SOURCE" ]]; then
    echo "[ERROR] nprobe binary not found." >&2
    exit 1
fi

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

install -m 755 "$SOURCE" "$INSTALL_DIR/nprobe"

mkdir -p "${XDG_CONFIG_HOME:-$HOME/.config}/netprobe"

echo "[OK] NetProbe installed: $INSTALL_DIR/nprobe"
"$INSTALL_DIR/nprobe" --version

echo "Ensure ~/.local/bin is in your PATH."
