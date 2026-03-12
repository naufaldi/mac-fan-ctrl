# FanGuard Release Guide

How to publish a new release of FanGuard via GitHub Releases and Homebrew.

## Prerequisites

### One-time setup

1. **Generate Tauri updater signing keys**:
   ```bash
   cargo tauri signer generate -w ~/.tauri/fanguard.key
   ```
   This outputs a public key and a base64 private key.

2. **Add GitHub secrets** (Settings > Secrets and variables > Actions):
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

4. **Create Homebrew tap repo**:
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
- Creates a draft GitHub Release with the `.dmg` attached
- Generates `latest.json` for the auto-updater
- Marks it as a prerelease (for beta tags)

### 4. Publish the draft release

1. Go to [GitHub Releases](https://github.com/naufaldi/mac-fan-ctrl/releases)
2. Find the draft release
3. Review the attached assets (should include `.dmg` and `latest.json`)
4. Edit release notes if needed
5. Click "Publish release"

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
```bash
brew tap naufaldi/tap
brew install --cask fanguard
```

### Upgrade via Homebrew
```bash
brew upgrade --cask fanguard
```

## Notes

- The app is currently **ad-hoc signed** (`signingIdentity: "-"`). Users must right-click > Open on first launch to bypass Gatekeeper.
- For proper Gatekeeper support, an Apple Developer ID certificate ($99/year) is needed for code signing and notarization.
- The `latest.json` file uploaded to releases enables the in-app auto-updater to detect new versions.
