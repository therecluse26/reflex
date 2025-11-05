class Reflex < Formula
  desc "Local-first, structure-aware code search engine for AI agents"
  homepage "https://github.com/reflex-search/reflex"
  version "0.2.10"
  license "MIT OR Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/reflex-search/reflex/releases/download/v0.2.10/rfx-macos-arm64"
      sha256 "123a370c4447790417ba2da1b3830defbdd83660ce4aa8b80f0a69d8fc28c3a2"
    elsif Hardware::CPU.intel?
      url "https://github.com/reflex-search/reflex/releases/download/v0.2.10/rfx-macos-x64"
      sha256 "a975af664f1f7898c4bc686dbc224e38a65bb7e18da12f43cb145ca1384a5453"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/reflex-search/reflex/releases/download/v0.2.10/rfx-linux-arm64"
      sha256 "059e2f1c32673f4dfa368b147ffc5cf0c582fcbb9c33e1c741f309cca40cd31c"
    elsif Hardware::CPU.intel?
      url "https://github.com/reflex-search/reflex/releases/download/v0.2.10/rfx-linux-x64"
      sha256 "f97d4b11009f44fbae25ab39c1441a2a3197c2f94cc0b5e79718c8bf7377a6f1"
    end
  end

  def install
    # The downloaded file is already the binary, just rename and install it
    if OS.mac?
      if Hardware::CPU.arm?
        bin.install "rfx-macos-arm64" => "rfx"
      elsif Hardware::CPU.intel?
        bin.install "rfx-macos-x64" => "rfx"
      end
    elsif OS.linux?
      if Hardware::CPU.arm?
        bin.install "rfx-linux-arm64" => "rfx"
      elsif Hardware::CPU.intel?
        bin.install "rfx-linux-x64" => "rfx"
      end
    end
  end

  test do
    system "#{bin}/rfx", "--version"
  end
end
