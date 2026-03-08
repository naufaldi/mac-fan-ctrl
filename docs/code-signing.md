# Code Signing & Notarization

## Overview

mac-fan-ctrl requires IOKit/SMC access, which is incompatible with the Mac App Store sandbox. Distribution uses Developer ID signing + notarization.

## Signing Modes

### Ad-hoc (local development)

The default config uses `"-"` as `signingIdentity`, which produces an ad-hoc signature. This works for local development and testing but will trigger Gatekeeper warnings on other machines.

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

After building with `pnpm tauri build`:

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

Tauri v2 supports automatic notarization when these env vars are set during `pnpm tauri build`. See [Tauri Code Signing docs](https://tauri.app/distribute/sign/macos/).

## Entitlements

The `Entitlements.plist` disables App Sandbox (`com.apple.security.app-sandbox = false`) because the app requires:

- Direct IOKit access to `AppleSMC` for reading/writing SMC keys
- Root privilege escalation for fan control writes

## Mac App Store Incompatibility

This app cannot be distributed via the Mac App Store because:

1. SMC writes require root privileges
2. IOKit direct access is blocked by App Sandbox
3. The app uses `macOSPrivateApi` for tray-only mode
