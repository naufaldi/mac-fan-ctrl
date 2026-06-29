# Code Signing & Notarization

## Overview

FanGuard now has two macOS distribution paths:

- **Developer ID/Homebrew**: direct distribution with fan control, the privileged helper, GitHub updater, and notarization.
- **TestFlight/App Store Connect**: sandboxed, monitoring-only distribution. Fan write actions, privileged helper install, LaunchDaemon setup, root/admin escalation, and the self-updater are disabled or hidden.

## Signing Modes

### Ad-hoc (local development)

Use `"-"` as `signingIdentity` only when you intentionally want an ad-hoc local build. This works for local development and testing but will trigger Gatekeeper warnings on other machines.

### Developer ID (distribution)

For distribution, you need an Apple Developer account ($99/year).

1. Enroll at [developer.apple.com](https://developer.apple.com)
2. Create a "Developer ID Application" certificate in Xcode or the Developer portal
3. Set the signing identity in `tauri.conf.json`:

```json
"macOS": {
    "entitlements": "Entitlements.plist",
    "signingIdentity": "Developer ID Application: Your Name (TEAM_ID)"
}
```

Or use environment variables for CI:

```bash
export APPLE_SIGNING_IDENTITY="Developer ID Application: Your Name (TEAM_ID)"
```

## Notarization

Apple requires notarization for all Developer ID-signed apps distributed outside the Mac App Store.

### Prerequisites

- Apple Developer account
- App-specific password (generate at [appleid.apple.com](https://appleid.apple.com))
- Team ID (visible in Developer portal)

### Environment Variables

```bash
export APPLE_ID="your@email.com"
export APPLE_PASSWORD="app-specific-password"
export APPLE_TEAM_ID="YOUR_TEAM_ID"
```

### Manual Notarization

After building with `pnpm tauri:build:direct`:

```bash
# Submit for notarization
xcrun notarytool submit \
    src-tauri/target/release/bundle/macos/mac-fan-ctrl.app.zip \
    --apple-id "$APPLE_ID" \
    --password "$APPLE_PASSWORD" \
    --team-id "$APPLE_TEAM_ID" \
    --wait

# Staple the notarization ticket
xcrun stapler staple \
    src-tauri/target/release/bundle/macos/mac-fan-ctrl.app
```

### Tauri Notarization (CI)

Tauri v2 supports automatic notarization when these env vars are set during `pnpm tauri:build:direct`. See [Tauri Code Signing docs](https://tauri.app/distribute/sign/macos/).

## Entitlements

The direct distribution `Entitlements.plist` disables App Sandbox (`com.apple.security.app-sandbox = false`) because fan control requires:

- Direct IOKit access to `AppleSMC` for reading/writing SMC keys
- Root privilege escalation for fan control writes

The TestFlight distribution uses `Entitlements.testflight.plist` with App Sandbox enabled:

```xml
<key>com.apple.security.app-sandbox</key>
<true/>
```

## TestFlight Build

Use the TestFlight config merge and Rust feature:

```bash
pnpm tauri:build:testflight
```

This runs:

```bash
tauri build --features app-store --config src-tauri/tauri.testflight.conf.json --bundles app --ci
```

The TestFlight config:

- enables App Sandbox via `Entitlements.testflight.plist`
- strips helper artifacts from the final `.app` bundle
- disables Tauri `macOSPrivateApi`
- removes updater permissions from the TestFlight window capability
- builds the frontend with `VITE_FANGUARD_DISTRIBUTION=app-store`

## Mac App Store Constraints

The direct fan-control feature set is not shipped to TestFlight because:

1. SMC writes require root privileges
2. LaunchDaemon/helper installation requires admin escalation
3. App Store builds must use App Sandbox
4. App Store updates are managed through App Store Connect, not the GitHub updater

The TestFlight path keeps read-only monitoring and hides unsupported write controls.
