# FanGuard Release Guide

How to publish a new release of FanGuard via GitHub Releases and Homebrew.

## Prerequisites

### One-time setup

1. **Generate Tauri updater signing keys**:
   ```bash
   cargo tauri signer generate -w ~/.tauri/fanguard.key
   ```
   This outputs a public key and a base64 private key.

2. **Add Tauri updater GitHub secrets** (Settings > Secrets and variables > Actions):
   - `TAURI_SIGNING_PRIVATE_KEY` — the base64 private key from step 1
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — the password you chose (or empty)

3. **Update pubkey in config** — paste the public key into `src-tauri/tauri.conf.json`:
   ```json
   "plugins": {
     "updater": {
       "pubkey": "<YOUR_REAL_PUBLIC_KEY>"
     }
   }
   ```

4. **Add Apple signing and notarization GitHub secrets**:
   - `APPLE_CERTIFICATE` — base64 encoded Developer ID Application `.p12`
   - `APPLE_CERTIFICATE_PASSWORD` — password used when exporting the `.p12`
   - `KEYCHAIN_PASSWORD` — random password for the temporary CI keychain
   - `APPLE_SIGNING_IDENTITY` — `Developer ID Application: Naufaldi Rafif Satriya (J7C53T6UGG)`
   - `APPLE_ID` — Apple ID email used for notarization
   - `APPLE_PASSWORD` — Apple app-specific password
   - `APPLE_TEAM_ID` — Apple Developer Team ID

5. **Create Homebrew tap repo**:
   - Create a new public GitHub repo: `naufaldi/homebrew-tap`
   - Copy `homebrew-cask/Casks/fanguard.rb` into it at `Casks/fanguard.rb`
   - Commit and push

## Publishing a Release

### 1. Bump version

Update version in three files (keep them in sync):
- `src-tauri/tauri.conf.json` → `"version"`
- `src-tauri/Cargo.toml` → `version`
- `package.json` → `"version"`

For beta releases use: `0.1.0-beta.1`, `0.1.0-beta.2`, etc.
For stable releases use: `1.0.0`, `1.1.0`, etc.

### 2. Commit and tag

```bash
git add -A
git commit -m "release: v0.1.0-beta.1"
git tag v0.1.0-beta.1
git push origin main
git push origin v0.1.0-beta.1
```

### 3. GitHub Actions builds automatically

The `release.yml` workflow triggers on any `v*` tag push. It:
- Builds a universal macOS binary (Intel + Apple Silicon)
- Imports the Developer ID certificate into a temporary CI keychain
- Signs, notarizes, and staples the macOS app bundle
- Creates a draft GitHub Release with the `.dmg` attached
- Generates `latest.json` for the auto-updater
- Marks it as a prerelease (for beta tags)

### 4. Publish the draft release

1. Go to [GitHub Releases](https://github.com/naufaldi/mac-fan-ctrl/releases)
2. Find the draft release
3. Review the attached assets (should include `.dmg` and `latest.json`)
4. Download the `.dmg` and verify Gatekeeper acceptance before publishing:
   ```bash
   codesign --verify --deep --strict --verbose=4 /Applications/FanGuard.app
   spctl --assess --type execute --verbose=4 /Applications/FanGuard.app
   xcrun stapler validate -v /Applications/FanGuard.app
   ```
5. Edit release notes if needed
6. Click "Publish release"

### 5. Update Homebrew cask

After the release is published:

```bash
# Download the DMG and compute SHA256
curl -L -o FanGuard.dmg "https://github.com/naufaldi/mac-fan-ctrl/releases/download/v0.1.0-beta.1/FanGuard_0.1.0-beta.1_universal.dmg"
shasum -a 256 FanGuard.dmg
```

Then update `Casks/fanguard.rb` in the `naufaldi/homebrew-tap` repo:

```ruby
cask "fanguard" do
  version "0.1.0-beta.1"
  sha256 "<paste-sha256-here>"
  # ... rest unchanged
end
```

Commit and push to the homebrew-tap repo.

## User Installation

### Direct download
```
Download from: https://github.com/naufaldi/mac-fan-ctrl/releases
```

### Homebrew

Homebrew 5+ enforces tap trust for non-official taps (`$HOMEBREW_REQUIRE_TAP_TRUST`).
Users must trust the tap once before install or upgrade:

```bash
brew tap naufaldi/tap
brew trust naufaldi/tap
brew install --cask fanguard
```

### Upgrade via Homebrew

```bash
brew upgrade --cask fanguard
```

If upgrade is skipped with "tap trust is required", run `brew trust naufaldi/tap` first.

## Notes

- Release artifacts must be Developer ID signed and notarized before publishing. Gatekeeper should report `accepted`.
- For proper Gatekeeper support, the Apple Developer ID certificate must be exported with its private key as a `.p12` for CI.
- The `latest.json` file uploaded to releases enables the in-app auto-updater to detect new versions.
