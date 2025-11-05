# Homebrew Tap for Reflex

This is the Homebrew tap for [Reflex](https://github.com/reflex-search/reflex), a local-first, structure-aware code search engine for AI agents.

## Installation

```bash
brew tap reflex-search/reflex
brew install reflex
```

Or in one line:

```bash
brew install reflex-search/reflex/reflex
```

## Verify Installation

```bash
rfx --version
```

## Initial Setup (One-Time)

After creating this repository, run the helper script to populate SHA256 hashes:

```bash
./update-formula.sh 0.2.10
git add Formula/reflex.rb
git commit -m "Add SHA256 hashes for v0.2.10"
git push
```

## Auto-Update

This formula is automatically updated by GitHub Actions whenever a new release is published to the main Reflex repository (via the `dawidd6/action-homebrew-bump-formula` action).
