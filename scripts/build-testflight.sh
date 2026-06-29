#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_BUNDLE="$ROOT_DIR/src-tauri/target/release/bundle/macos/FanGuard.app"
TEMP_HELPER="$ROOT_DIR/src-tauri/target/release/fanguard-helper"

rm -rf "$APP_BUNDLE"
rm -f "$TEMP_HELPER" "$ROOT_DIR/src-tauri/target/release/fanguard-helper.d"
mkdir -p "$(dirname "$TEMP_HELPER")"

# Tauri v2 stages every Cargo binary target from this package. The real
# privileged helper is direct-distribution only, so TestFlight gives Tauri a
# temporary placeholder, removes it from the bundle, then signs the final app.
printf '#!/bin/sh\nexit 1\n' > "$TEMP_HELPER"
chmod 755 "$TEMP_HELPER"

cleanup() {
	rm -f "$TEMP_HELPER" "$ROOT_DIR/src-tauri/target/release/fanguard-helper.d"
}
trap cleanup EXIT

cd "$ROOT_DIR"
tauri build --features app-store --config src-tauri/tauri.testflight.conf.json --bundles app --ci

rm -f "$APP_BUNDLE/Contents/MacOS/fanguard-helper"

codesign --force --deep --sign - \
	--entitlements "$ROOT_DIR/src-tauri/Entitlements.testflight.plist" \
	"$APP_BUNDLE"

if find "$APP_BUNDLE" -name '*helper*' -o -name '*LaunchDaemon*' | grep -q .; then
	echo "TestFlight bundle still contains helper artifacts" >&2
	exit 1
fi

echo "TestFlight app ready: $APP_BUNDLE"
