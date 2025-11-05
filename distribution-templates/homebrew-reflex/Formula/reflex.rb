class Reflex < Formula
  desc "Local-first, structure-aware code search engine for AI agents"
  homepage "https://github.com/reflex-search/reflex"
  version "0.2.10"
  license "MIT OR Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/reflex-search/reflex/releases/download/v0.2.10/rfx-macos-arm64"
      sha256 ""  # Will be auto-updated by GitHub Actions
    else
      url "https://github.com/reflex-search/reflex/releases/download/v0.2.10/rfx-macos-x64"
      sha256 ""  # Will be auto-updated by GitHub Actions
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/reflex-search/reflex/releases/download/v0.2.10/rfx-linux-arm64"
      sha256 ""  # Will be auto-updated by GitHub Actions
    else
      url "https://github.com/reflex-search/reflex/releases/download/v0.2.10/rfx-linux-x64"
      sha256 ""  # Will be auto-updated by GitHub Actions
    end
  end

  def install
    # The downloaded file is already the binary, just rename and install it
    if OS.mac?
      if Hardware::CPU.arm?
        bin.install "rfx-macos-arm64" => "rfx"
      else
        bin.install "rfx-macos-x64" => "rfx"
      end
    else
      if Hardware::CPU.arm?
        bin.install "rfx-linux-arm64" => "rfx"
      else
        bin.install "rfx-linux-x64" => "rfx"
      end
    end
  end

  test do
    system "#{bin}/rfx", "--version"
  end
end
