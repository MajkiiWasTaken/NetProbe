#!/usr/bin/env bash
# /************************************************
# * File: uninstall.sh
# * Author: Michal Švrček
# *
# * NetProbe Linux uninstaller.
# *
# * ver. 0.6.0
# *************************************************/

set -euo pipefail

rm -f -- "$HOME/.local/bin/nprobe"

echo "[OK] NetProbe uninstalled."
echo "Saved device profiles were preserved."
