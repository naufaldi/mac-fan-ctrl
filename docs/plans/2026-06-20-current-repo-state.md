# Current Repo State

**Captured:** 2026-06-20 (session audit)

## Summary

Local `main` is fully synced with `origin/main`. Release `v0.1.0-beta.3` is published. The external Homebrew tap already points to beta.3. Nothing needs to be pushed from this repo for the current release.

## Git State

| Item | Value |
|------|-------|
| Branch | `main` |
| HEAD | `d9272ec8d8eeb4bf5c5e0fc068d36dfd0a84830b` |
| Tag | `v0.1.0-beta.3` |
| Upstream | `origin/main` |
| Ahead / behind | `0 / 0` |

Latest commit on `main`:

```
d9272ec fix: restore CI tests and add x86_64 helper stub for release build
```

## Release State

| Item | Value |
|------|-------|
| Version | `0.1.0-beta.3` |
| GitHub release | Published (prerelease) |
| URL | https://github.com/naufaldi/mac-fan-ctrl/releases/tag/v0.1.0-beta.3 |
| DMG asset | `FanGuard_0.1.0-beta.3_universal.dmg` |
| DMG SHA256 | `b6af7f31344a7de6cf4d2abc9a605de96d1ea63149e42bd78ff21c181894847c` |

Version is consistent across:

- [`package.json`](../../package.json)
- [`src-tauri/Cargo.toml`](../../src-tauri/Cargo.toml)
- [`src-tauri/tauri.conf.json`](../../src-tauri/tauri.conf.json)

## Homebrew State

External tap `naufaldi/homebrew-tap` already has `Casks/fanguard.rb` pinned to `0.1.0-beta.3` with the same SHA256 as the published DMG.

Local copy at [`homebrew-cask/Casks/fanguard.rb`](../../homebrew-cask/Casks/fanguard.rb) matches that state but is **modified in the working tree** (uncommitted). No separate push is needed unless you want to commit the local mirror for repo consistency.

Install docs (Homebrew 5+ tap trust) live in:

- [`docs/release-guide.md`](../release-guide.md)
- [`README.md`](../../README.md)
- [`homebrew-cask/README.md`](../../homebrew-cask/README.md)

## Local Working Tree (Pre-existing)

These existed before the status note and are **not** part of the beta.3 release push:

| Path | Status | Notes |
|------|--------|-------|
| `homebrew-cask/Casks/fanguard.rb` | Modified | Duplicates published tap; safe to commit or discard |
| `src-tauri/icons/*` (android, ios, Square*, etc.) | Untracked | Tauri-generated icon bundle; decide keep vs `.gitignore` |
| `test-results/` | Untracked | Playwright failure artifacts; safe to delete locally |

Playwright failure snapshot (`test-results/hello-world-hello-world-ping-flow/error-context.md`):

```
Failed to connect to sensor backend
Cannot read properties of undefined (reading 'invoke')
```

Relates to open issue [#39](https://github.com/naufaldi/mac-fan-ctrl/issues/39) (E2E tests still cover hello-world, not dashboard).

## Open GitHub Items

| Type | # | Title | Notes |
|------|---|-------|-------|
| PR | [#60](https://github.com/naufaldi/mac-fan-ctrl/pull/60) | chore: pin Homebrew cask to v0.1.0-beta.2 | Stale, conflicting; superseded by beta.3 on `main` — close |
| Issue | [#39](https://github.com/naufaldi/mac-fan-ctrl/issues/39) | Update E2E tests to cover actual dashboard UI | |
| Issue | [#32](https://github.com/naufaldi/mac-fan-ctrl/issues/32) | Multi-point fan curve editor | Phase B |
| Issue | [#31](https://github.com/naufaldi/mac-fan-ctrl/issues/31) | Historical data storage and graphs | Phase B |

## Recent Work Context (beta.3)

Beta.3 shipped privileged-helper fixes so fan control works after install without running the GUI as root:

- Bundled `fanguard-helper` in release builds
- x86_64 helper stub for universal release CI
- Prefer root helper socket over unprivileged SMC writes (F0Md failures on fresh installs)

See also: [`docs/plans/2026-03-10-privileged-helper.md`](2026-03-10-privileged-helper.md)

## Recommended Follow-ups

1. **Close PR #60** — content already landed on `main` as beta.3.
2. **Clean local tree** — discard or commit `homebrew-cask/Casks/fanguard.rb`; remove `test-results/`.
3. **Icons** — either commit generated Tauri icons or add to `.gitignore` if build-only.
4. **E2E** — address #39 so Playwright runs against real Tauri invoke, not browser-only hello-world.

## Push Checklist (Next Release)

When cutting `v0.1.0-beta.4` or stable:

1. Bump version in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`
2. Commit, tag, push `main` and tag
3. Publish GitHub release (CI builds DMG)
4. Update SHA256 in `naufaldi/homebrew-tap` (and optionally local `homebrew-cask/Casks/fanguard.rb`)

Full procedure: [`docs/release-guide.md`](../release-guide.md)
