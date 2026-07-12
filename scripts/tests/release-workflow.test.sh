#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
WORKFLOW="$PROJECT_ROOT/.github/workflows/release.yml"

grep -Fq 'tauriScript: ./scripts/tauri-release-build.sh' "$WORKFLOW" || {
	printf 'FAIL: release workflow must build through the DMG verification wrapper\n' >&2
	exit 1
}

printf 'PASS: release workflow uses the verified build wrapper\n'
