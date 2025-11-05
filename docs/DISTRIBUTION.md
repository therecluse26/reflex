# Distribution Setup Guide

This document explains how to set up automated publishing to 5 package managers via GitHub Actions.

## Overview

When you push a git tag (e.g., `v0.2.11`), GitHub Actions will automatically:

1. **Publish to crates.io** (Rust package registry)
2. **Create WinGet PR** to microsoft/winget-pkgs (Windows Package Manager)
3. **Update Scoop bucket** (Windows developer package manager)
4. **Update Homebrew tap** (macOS/Linux package manager)
5. **Update AUR package** (Arch Linux User Repository)

## One-Time Setup

### Step 1: Configure crates.io Trusted Publishing

1. Go to https://crates.io/me
2. Click on your profile → "Trusted Publishing"
3. Add a new trusted publisher:
   - **Repository**: `reflex-search/reflex`
   - **Workflow**: `publish-packages.yml`
   - **Environment**: Leave blank (or use `release` if you add an environment)
4. This eliminates the need for API tokens!

### Step 2: Create External Repositories

You need to create two new GitHub repositories:

#### A. Scoop Bucket Repository

1. Create new repo: `reflex-search/scoop-reflex`
2. Copy files from `distribution-templates/scoop-reflex/` to the new repo:
   ```bash
   cp -r distribution-templates/scoop-reflex/* /path/to/scoop-reflex/
   cd /path/to/scoop-reflex
   git add .
   git commit -m "Initial Scoop bucket"
   git push
   ```
3. Enable GitHub Actions in the repository settings

#### B. Homebrew Tap Repository

1. Create new repo: `reflex-search/homebrew-reflex`
2. Copy files from `distribution-templates/homebrew-reflex/` to the new repo:
   ```bash
   cp -r distribution-templates/homebrew-reflex/* /path/to/homebrew-reflex/
   cd /path/to/homebrew-reflex
   git add .
   git commit -m "Initial Homebrew tap"
   git push
   ```

### Step 3: Create GitHub Secrets

Add these secrets to your **main reflex repository** (Settings → Secrets and variables → Actions):

#### Required Secrets:

1. **WINGET_TOKEN**
   - Create a GitHub Personal Access Token (PAT)
   - Go to: https://github.com/settings/tokens/new
   - Scopes needed: `public_repo`, `workflow`
   - Description: "WinGet Releaser"
   - Copy the token and add as secret

2. **SCOOP_TOKEN**
   - Create another GitHub PAT (or reuse WINGET_TOKEN)
   - Scopes needed: `repo`, `workflow`
   - This is used to update the scoop-reflex repository

3. **HOMEBREW_TAP_TOKEN**
   - Create another GitHub PAT (or reuse previous)
   - Scopes needed: `repo`, `workflow`
   - This is used to update the homebrew-reflex repository

4. **AUR_USERNAME**
   - Your AUR account username
   - Create account at: https://aur.archlinux.org/register

5. **AUR_EMAIL**
   - Your AUR account email address

6. **AUR_SSH_PRIVATE_KEY**
   - Generate SSH key pair:
     ```bash
     ssh-keygen -t ed25519 -C "aur@reflex-search" -f ~/.ssh/aur
     ```
   - Add public key to AUR: https://aur.archlinux.org/account/
   - Copy private key contents to this secret:
     ```bash
     cat ~/.ssh/aur  # Copy this entire output
     ```

### Step 4: Create AUR Package

1. Clone the AUR repository (first time only):
   ```bash
   git clone ssh://aur@aur.archlinux.org/reflex-bin.git
   cd reflex-bin
   ```

2. Copy initial files:
   ```bash
   cp /path/to/reflex/aur/PKGBUILD .
   cp /path/to/reflex/aur/.SRCINFO .
   ```

3. Update checksums:
   ```bash
   updpkgsums
   makepkg --printsrcinfo > .SRCINFO
   ```

4. Commit and push:
   ```bash
   git add PKGBUILD .SRCINFO
   git commit -m "Initial commit: reflex-bin 0.2.10"
   git push
   ```

After this initial setup, GitHub Actions will handle all future updates!

### Step 5: Update WinGet Manifest Identifier

Before first release, you need to claim your WinGet package identifier:

1. Go to the publish-packages.yml workflow
2. Update the identifier if needed:
   ```yaml
   identifier: reflex-search.reflex  # Change if you want different identifier
   ```

## Publishing a New Release

Once setup is complete, publishing is simple:

```bash
# 1. Update version in Cargo.toml
vim Cargo.toml  # Change version = "0.2.10" to "0.2.11"

# 2. Commit and tag
git add Cargo.toml
git commit -m "chore: bump version to 0.2.11"
git push origin main

# 3. Tag and push
git tag v0.2.11
git push origin v0.2.11
```

**GitHub Actions will automatically:**
- Build binaries (via existing release.yml workflow)
- Create GitHub Release with binaries
- Publish to crates.io
- Create WinGet PR (requires Microsoft review, usually ~1-3 days)
- Update Scoop bucket manifest
- Update Homebrew tap formula
- Update AUR package

## Installation for Users

After your first successful publication, users can install Reflex via:

```bash
# Rust developers (any OS)
cargo install reflex

# macOS/Linux (Homebrew)
brew tap reflex-search/reflex
brew install reflex

# Windows 11 (WinGet - built-in)
winget install reflex-search.reflex

# Windows (Scoop - developers)
scoop bucket add reflex https://github.com/reflex-search/scoop-reflex
scoop install reflex

# Arch Linux
yay -S reflex-bin
# or
paru -S reflex-bin
```

## Troubleshooting

### crates.io publish fails

- Check that Trusted Publishing is configured correctly at https://crates.io/me
- Verify the repository name matches exactly: `reflex-search/reflex`
- Ensure workflow name is correct: `publish-packages.yml`

### WinGet publish fails

- Check WINGET_TOKEN has correct scopes (`public_repo`, `workflow`)
- First-time submissions may be rejected - Microsoft needs to review
- After first acceptance, updates are usually fast

### Scoop/Homebrew update fails

- Verify the SCOOP_TOKEN and HOMEBREW_TAP_TOKEN have repo access
- Check that the external repositories exist and are public
- Ensure GitHub Actions are enabled in those repos

### AUR publish fails

- Verify SSH key is correctly added to AUR account
- Check AUR_USERNAME and AUR_EMAIL match your account
- Ensure the AUR package name `reflex-bin` is available
- Test SSH connection: `ssh aur@aur.archlinux.org help`

## Manual Testing

You can manually trigger the workflow to test:

1. Go to Actions → "Publish Packages"
2. Click "Run workflow"
3. Enter a tag name (e.g., `v0.2.10`)
4. Click "Run workflow"

This is useful for testing without creating a new release.

## Security Notes

- Never commit API tokens or SSH keys to git
- Use GitHub Secrets for all sensitive values
- Trusted Publishing for crates.io is more secure than using static tokens
- Regularly rotate your GitHub PATs (every 6-12 months)
- Use fine-grained PATs when available for better security

## Monitoring

After pushing a tag:

1. Watch GitHub Actions: https://github.com/reflex-search/reflex/actions
2. Check crates.io: https://crates.io/crates/reflex
3. Monitor WinGet PR: https://github.com/microsoft/winget-pkgs/pulls
4. Verify Scoop bucket: https://github.com/reflex-search/scoop-reflex
5. Verify Homebrew tap: https://github.com/reflex-search/homebrew-reflex
6. Check AUR package: https://aur.archlinux.org/packages/reflex-bin

## Questions?

- **GitHub Actions not running?** Check workflow permissions in repo settings
- **Secrets not working?** Verify secret names match exactly (case-sensitive)
- **Need help?** Open an issue at https://github.com/reflex-search/reflex/issues
