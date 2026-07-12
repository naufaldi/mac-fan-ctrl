#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT_PATH="$PROJECT_ROOT/scripts/build-local-notarized.sh"

fail() {
	printf 'FAIL: %s\n' "$1" >&2
	exit 1
}

assert_contains() {
	local output="$1"
	local expected="$2"

	[[ "$output" == *"$expected"* ]] || fail "expected output to contain: $expected"
}

test_rejects_multiple_dmg_candidates() {
	local temp_dir
	temp_dir="$(mktemp -d)"
	trap 'rm -rf "$temp_dir"' RETURN
	mkdir -p "$temp_dir/home/.config/fanguard" "$temp_dir/bundle" "$temp_dir/bin"
	printf '%s\n' \
		'APPLE_ID=test@example.com' \
		'APPLE_PASSWORD=test-password' \
		'APPLE_TEAM_ID=TESTTEAM' \
		>"$temp_dir/home/.config/fanguard/notarization.env"
	touch "$temp_dir/bundle/first.dmg" "$temp_dir/bundle/second.dmg"
	printf '%s\n' '#!/usr/bin/env bash' 'printf "%s\n" "1) test Developer ID Application"' >"$temp_dir/bin/security"
	chmod +x "$temp_dir/bin/security"

	local output
	local status
	set +e
	output="$(
		HOME="$temp_dir/home" \
		PATH="$temp_dir/bin:$PATH" \
		FANGUARD_SKIP_BUILD=1 \
		FANGUARD_DMG_DIR="$temp_dir/bundle" \
		bash "$SCRIPT_PATH" 2>&1
	)"
	status=$?
	set -e

	[[ "$status" -ne 0 ]] || fail "multiple DMGs must return a non-zero exit code"
	assert_contains "$output" "Expected exactly one DMG"
}

test_detaches_verified_dmg() {
	local temp_dir
	temp_dir="$(mktemp -d)"
	trap 'rm -rf "$temp_dir"' RETURN
	mkdir -p "$temp_dir/bin" "$temp_dir/bundle"
	touch "$temp_dir/bundle/FanGuard.dmg"

	printf '%s\n' '#!/usr/bin/env bash' 'printf "%s\n" "1) test Developer ID Application"' >"$temp_dir/bin/security"
	printf '%s\n' '#!/usr/bin/env bash' 'exit 0' >"$temp_dir/bin/xcrun"
	printf '%s\n' '#!/usr/bin/env bash' 'exit 0' >"$temp_dir/bin/codesign"
	printf '%s\n' '#!/usr/bin/env bash' 'exit 0' >"$temp_dir/bin/spctl"
	printf '%s\n' \
		'#!/usr/bin/env bash' \
		'if [[ "$1" == "attach" ]]; then' \
		'  mount_dir="${@: -1}"' \
		'  mkdir -p "$mount_dir/FanGuard.app/Contents/MacOS"' \
		'  touch "$mount_dir/FanGuard.app/Contents/MacOS/fanguard-helper"' \
		'fi' \
		'[[ "$1" != "detach" ]] || printf "detached\n" >>"$FANGUARD_TEST_LOG"' \
		>"$temp_dir/bin/hdiutil"
	chmod +x "$temp_dir/bin/"*

	APPLE_ID=test@example.com \
	APPLE_PASSWORD=test-password \
	APPLE_TEAM_ID=TESTTEAM \
	FANGUARD_DMG_DIR="$temp_dir/bundle" \
	FANGUARD_TEST_LOG="$temp_dir/hdiutil.log" \
	PATH="$temp_dir/bin:$PATH" \
		bash "$PROJECT_ROOT/scripts/notarize-dmg.sh" >/dev/null

	[[ "$(<"$temp_dir/hdiutil.log")" == "detached" ]] || fail "verified DMG must be detached during cleanup"
}

test_rejects_multiple_dmg_candidates
test_detaches_verified_dmg
printf 'PASS: build-local-notarized tests\n'
