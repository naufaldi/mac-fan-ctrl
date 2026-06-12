# FanGuard Homebrew Tap

Personal Homebrew tap for [FanGuard](https://github.com/naufaldi/mac-fan-ctrl).

Live tap repo: https://github.com/naufaldi/homebrew-tap

## Install

Homebrew 5+ requires trusting third-party taps before install:

```bash
brew tap naufaldi/tap
brew trust naufaldi/tap
brew install --cask fanguard
```

## Upgrade

```bash
brew upgrade --cask fanguard
```

If you see `tap trust is required`, run `brew trust naufaldi/tap` and retry.

## Maintainer sync

On each release, copy `Casks/fanguard.rb` from this directory to `naufaldi/homebrew-tap`, update `version` and `sha256`, then push.
