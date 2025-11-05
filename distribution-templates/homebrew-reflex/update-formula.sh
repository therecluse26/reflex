#!/usr/bin/env bash
# Helper script to update the Homebrew formula with correct SHA256 hashes
# Run this after creating a new release

set -e

VERSION="${1:-0.2.10}"
REPO="reflex-search/reflex"

echo "Updating Homebrew formula for version $VERSION..."

# Download binaries and calculate SHA256
echo "Calculating SHA256 hashes..."

echo -n "macOS ARM64: "
MAC_ARM64_SHA=$(curl -sL "https://github.com/${REPO}/releases/download/v${VERSION}/rfx-macos-arm64" | shasum -a 256 | cut -d' ' -f1)
echo "$MAC_ARM64_SHA"

echo -n "macOS x64: "
MAC_X64_SHA=$(curl -sL "https://github.com/${REPO}/releases/download/v${VERSION}/rfx-macos-x64" | shasum -a 256 | cut -d' ' -f1)
echo "$MAC_X64_SHA"

echo -n "Linux ARM64: "
LINUX_ARM64_SHA=$(curl -sL "https://github.com/${REPO}/releases/download/v${VERSION}/rfx-linux-arm64" | shasum -a 256 | cut -d' ' -f1)
echo "$LINUX_ARM64_SHA"

echo -n "Linux x64: "
LINUX_X64_SHA=$(curl -sL "https://github.com/${REPO}/releases/download/v${VERSION}/rfx-linux-x64" | shasum -a 256 | cut -d' ' -f1)
echo "$LINUX_X64_SHA"

# Update formula
echo ""
echo "Updating Formula/reflex.rb..."

sed -i.bak \
  -e "s/version \".*\"/version \"$VERSION\"/" \
  -e "s|download/v[^/]*/|download/v$VERSION/|g" \
  -e "s/PLACEHOLDER_ARM64_MAC_SHA256/$MAC_ARM64_SHA/" \
  -e "s/PLACEHOLDER_X64_MAC_SHA256/$MAC_X64_SHA/" \
  -e "s/PLACEHOLDER_ARM64_LINUX_SHA256/$LINUX_ARM64_SHA/" \
  -e "s/PLACEHOLDER_X64_LINUX_SHA256/$LINUX_X64_SHA/" \
  Formula/reflex.rb

rm Formula/reflex.rb.bak

echo "✅ Formula updated successfully!"
echo ""
echo "Review the changes and commit:"
echo "  git diff Formula/reflex.rb"
echo "  git add Formula/reflex.rb"
echo "  git commit -m 'Update to version $VERSION'"
echo "  git push"
