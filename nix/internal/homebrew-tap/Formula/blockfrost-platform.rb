class BlockfrostPlatform < Formula
  desc "Blockfrost platform is a software that services a developer-friendly JSON API for the Cardano ecosystem"
  homepage "https://platform.blockfrost.io/"
  license "Apache-2.0"
  version "@version@"

  if OS.mac?
    if Hardware::CPU.intel?
      url "@url_x86_64@"
      sha256 "@sha256_x86_64@"
    else
      url "@url_aarch64@"
      sha256 "@sha256_aarch64@"
    end
  end

  def install
    bin.install Dir["bin/*"]
    libexec.install Dir["libexec/*"]
  end

  test do
    system "#{bin}/blockfrost-platform", "--version"
  end
end
