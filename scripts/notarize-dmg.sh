#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DMG_DIR="${FANGUARD_DMG_DIR:-$PROJECT_ROOT/src-tauri/target/release/bundle/dmg}"
MOUNT_DIR=""

require_env() {
	local name="$1"
	[[ -n "${!name:-}" ]] || { printf 'Missing required environment variable: %s\n' "$name" >&2; exit 1; }
}

find_only() {
	local label="$1"
	local directory="$2"
	local paths=()
	shopt -s nullglob
	case "$label" in
		DMG) paths=("$directory"/*.dmg) ;;
		app) paths=("$directory"/*.app) ;;
		*) printf 'Unsupported artifact label: %s\n' "$label" >&2; exit 1 ;;
	esac
	shopt -u nullglob
	[[ "${#paths[@]}" -eq 1 ]] || { printf 'Expected exactly one %s in %s, found %s\n' "$label" "$directory" "${#paths[@]}" >&2; exit 1; }
	printf '%s\n' "${paths[0]}"
}

cleanup() {
	[[ -z "$MOUNT_DIR" ]] || hdiutil detach "$MOUNT_DIR" >/dev/null 2>&1 || true
	[[ -z "$MOUNT_DIR" ]] || rmdir "$MOUNT_DIR" 2>/dev/null || true
}

verify_app() {
	local app_path="$1"
	local helper_path="$app_path/Contents/MacOS/fanguard-helper"
	[[ -f "$helper_path" ]] || { printf 'Missing bundled helper: %s\n' "$helper_path" >&2; return 1; }
	codesign --verify --deep --strict --verbose=4 "$app_path"
	codesign --verify --strict --verbose=4 "$helper_path"
	spctl --assess --type execute --verbose=4 "$app_path"
	xcrun stapler validate -v "$app_path"
}

verify_dmg() {
	local dmg_path="$1"
	codesign --verify --strict --verbose=4 "$dmg_path"
	spctl --assess --type open --context context:primary-signature --verbose=4 "$dmg_path"
	xcrun stapler validate -v "$dmg_path"
}

require_env APPLE_ID
require_env APPLE_PASSWORD
require_env APPLE_TEAM_ID
security find-identity -v -p codesigning | grep -F "Developer ID Application" >/dev/null || {
	printf 'No valid Developer ID Application identity found in the current keychain search list\n' >&2
	exit 1
}

DMG_PATH="$(find_only DMG "$DMG_DIR")"
printf 'Submitting final DMG for notarization: %s\n' "$DMG_PATH"
xcrun notarytool submit "$DMG_PATH" \
	--apple-id "$APPLE_ID" \
	--password "$APPLE_PASSWORD" \
	--team-id "$APPLE_TEAM_ID" \
	--wait
xcrun stapler staple -v "$DMG_PATH"
verify_dmg "$DMG_PATH"

MOUNT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/fanguard-dmg.XXXXXX")"
trap cleanup EXIT
hdiutil attach "$DMG_PATH" -nobrowse -readonly -mountpoint "$MOUNT_DIR" >/dev/null
APP_PATH="$(find_only app "$MOUNT_DIR")"
verify_app "$APP_PATH"
printf 'Verified notarized FanGuard distribution: %s\n' "$DMG_PATH"
