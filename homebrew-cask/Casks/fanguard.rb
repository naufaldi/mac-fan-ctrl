cask "fanguard" do
  version "0.1.0-beta.2"
  sha256 "8014795b7edbcd0a56639f4292b5564c7403668b4da1e5da5491db1ce5a7d338"

  url "https://github.com/naufaldi/mac-fan-ctrl/releases/download/v#{version}/FanGuard_#{version}_universal.dmg"
  name "FanGuard"
  desc "macOS fan control utility — monitor temperatures and manage fan speeds via SMC"
  homepage "https://github.com/naufaldi/mac-fan-ctrl"

  livecheck do
    url :url
    strategy :github_latest
  end

  app "FanGuard.app"

  zap trash: [
    "~/.config/fanguard",
    "~/Library/Preferences/io.github.naufaldi.fanguard.plist",
    "~/Library/Saved Application State/io.github.naufaldi.fanguard.savedState",
  ]
end
