cask "fanguard" do
  version "0.1.0-beta.1"
  sha256 :no_check # Replace with real SHA256 after first release build

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
