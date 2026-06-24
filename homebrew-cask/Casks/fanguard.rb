cask "fanguard" do
  version "0.1.0-beta.3"
  sha256 "b6af7f31344a7de6cf4d2abc9a605de96d1ea63149e42bd78ff21c181894847c"

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
