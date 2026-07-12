#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CREDENTIALS_FILE="${FANGUARD_NOTARIZATION_ENV:-$HOME/.config/fanguard/notarization.env}"
DMG_DIR="${FANGUARD_DMG_DIR:-$PROJECT_ROOT/src-tauri/target/release/bundle/dmg}"

if [[ ! -f "$CREDENTIALS_FILE" ]]; then
	printf 'Missing notarization credentials: %s\n' "$CREDENTIALS_FILE" >&2
	exit 1
fi

# shellcheck source=/dev/null
set -a
source "$CREDENTIALS_FILE"
set +a

if [[ "${FANGUARD_SKIP_BUILD:-0}" != "1" ]]; then
	cd "$PROJECT_ROOT"
	pnpm tauri build
fi

FANGUARD_DMG_DIR="$DMG_DIR" "$PROJECT_ROOT/scripts/notarize-dmg.sh"
