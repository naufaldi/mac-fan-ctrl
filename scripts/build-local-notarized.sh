#!/usr/bin/env bash
set -euo pipefail

source "$HOME/.config/fanguard/notarization.env"

pnpm tauri build
