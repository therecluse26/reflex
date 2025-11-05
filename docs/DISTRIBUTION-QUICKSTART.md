# Distribution Quick Start

## TL;DR Setup Checklist

### 1. Configure crates.io (5 min)
- [ ] Go to https://crates.io/me → "Trusted Publishing"
- [ ] Add: `reflex-search/reflex` + `publish-packages.yml`

### 2. Create External Repos (10 min)
- [ ] Create `reflex-search/scoop-reflex`
  - Copy files from `distribution-templates/scoop-reflex/`
- [ ] Create `reflex-search/homebrew-reflex`
  - Copy files from `distribution-templates/homebrew-reflex/`

### 3. Add GitHub Secrets (15 min)
Go to repo Settings → Secrets → Actions, add:

- [ ] `WINGET_TOKEN` - GitHub PAT (scopes: `public_repo`, `workflow`)
- [ ] `SCOOP_TOKEN` - GitHub PAT (scopes: `repo`, `workflow`)
- [ ] `HOMEBREW_TAP_TOKEN` - GitHub PAT (scopes: `repo`, `workflow`)
- [ ] `AUR_USERNAME` - Your AUR username
- [ ] `AUR_EMAIL` - Your AUR email
- [ ] `AUR_SSH_PRIVATE_KEY` - Your AUR SSH private key

### 4. Setup AUR (15 min)
```bash
# Generate SSH key
ssh-keygen -t ed25519 -f ~/.ssh/aur

# Add public key to https://aur.archlinux.org/account/

# Clone and setup
git clone ssh://aur@aur.archlinux.org/reflex-bin.git
cd reflex-bin
cp /path/to/reflex/aur/PKGBUILD .
updpkgsums
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD .SRCINFO
git commit -m "Initial commit: reflex-bin 0.2.10"
git push
```

## Publishing (30 seconds)

```bash
# Update version
vim Cargo.toml  # Bump version

# Commit and tag
git add Cargo.toml
git commit -m "chore: bump version to X.Y.Z"
git push
git tag vX.Y.Z
git push --tags
```

**Done!** GitHub Actions publishes everywhere automatically.

## What Users Get

```bash
cargo install reflex                           # Rust (any OS)
brew install reflex-search/reflex/reflex       # macOS/Linux
winget install reflex-search.reflex            # Windows 11
scoop install reflex                           # Windows (Scoop)
yay -S reflex-bin                              # Arch Linux
```

## See Full Docs

For detailed instructions, troubleshooting, and security notes, see [DISTRIBUTION.md](DISTRIBUTION.md).
