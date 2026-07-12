#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$PROJECT_ROOT"
pnpm tauri "$@"

find_target() {
	[[ "$#" -gt 0 ]] || return 0
	[[ "$1" != "--target" ]] || { printf '%s\n' "${2:-}"; return 0; }
	shift
	find_target "$@"
}

TARGET="$(find_target "$@")"

if [[ -n "$TARGET" ]]; then
	FANGUARD_DMG_DIR="$PROJECT_ROOT/src-tauri/target/$TARGET/release/bundle/dmg" \
		"$PROJECT_ROOT/scripts/notarize-dmg.sh"
else
	"$PROJECT_ROOT/scripts/notarize-dmg.sh"
fi
