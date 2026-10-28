cask "novaflix" do
  version "0.1.0"
  sha256 "25a9614ecf9237a2f405b767c1036d6f6219538c5d9eadfd71d93e73a9098355"

  url "https://github.com/novatic-cu/NovaFlix/releases/download/v#{version}/NovaFlix_#{version}_universal.dmg",
      verified: "github.com/novatic-cu/NovaFlix/"
  name "NovaFlix"
  desc "Buscador de escritorio para catálogos multimedia públicos"
  homepage "https://github.com/novatic-cu/NovaFlix"

  livecheck do
    url :url
    strategy :github_latest
  end

  app "NovaFlix.app"

  zap trash: [
    "~/Library/Application Support/com.novatic.novaflix",
    "~/Library/Caches/com.novatic.novaflix",
    "~/Library/WebKit/com.novatic.novaflix",
  ]
end
