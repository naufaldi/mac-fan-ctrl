#!/usr/bin/env bash
set -euo pipefail

if [[ "${FANGUARD_HARDWARE_SMOKE:-}" != "1" ]]; then
	echo "Refusing to run: set FANGUARD_HARDWARE_SMOKE=1 to opt in to real fan writes."
	exit 1
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
	echo "Hardware smoke requires macOS."
	exit 1
fi

echo "[smoke] Preconditions:"
echo "  - Production-style FanGuard build installed"
echo "  - Privileged helper running at /var/run/fanguard.sock"
echo ""
echo "[smoke] Manual verification checklist (writes real SMC fan settings):"
echo "  1. Open FanGuard and confirm fan 0 is in Auto"
echo "  2. Set fan 0 to a safe midpoint RPM via Custom in the main window"
echo "  3. Confirm dashboard shows Custom active (not Auto)"
echo "  4. Open tray fan submenu — exactly one mode should appear selected"
echo "  5. Set fan 0 back to Auto from the main window"
echo "  6. Confirm dashboard and tray return to Auto-only selection"
echo ""
echo "[smoke] Always restore fans to Auto before closing the app."

if [[ ! -S /var/run/fanguard.sock ]]; then
	echo "[smoke] WARNING: helper socket missing at /var/run/fanguard.sock"
	exit 1
fi

echo "[smoke] Helper socket present — proceed with manual steps above."
