# Release Management

Reflex follows **semantic versioning** (SemVer) with a simple manual release workflow powered by cargo-dist.

## Semantic Versioning

Version format: `MAJOR.MINOR.PATCH` (e.g., `0.2.7`)

- **MAJOR**: Breaking changes (incompatible API changes)
- **MINOR**: New features (backward-compatible functionality)
- **PATCH**: Bug fixes (backward-compatible bug fixes)

**Examples:**
- `0.2.6 → 0.2.7`: Bug fix (PATCH bump)
- `0.2.7 → 0.3.0`: New feature like `--timeout` flag (MINOR bump)
- `0.3.0 → 1.0.0`: Breaking change or stable release (MAJOR bump)

## Creating a Release

**Simple 3-step process:**

```bash
# 1. Update version in Cargo.toml
vim Cargo.toml  # Change version = "0.2.6" to "0.2.7"

# 2. Commit and push
git add Cargo.toml
git commit -m "chore: bump version to 0.2.7"
git push origin main

# 3. Create and push tag
git tag v0.2.7
git push origin v0.2.7
```

**That's it!** When you push the tag, GitHub Actions automatically:
- Builds binaries for all platforms (Linux, macOS, Windows, ARM, x86_64)
- Extracts raw executables from cargo-dist archives
- Creates a GitHub Release with:
  - Raw binaries (e.g., `rfx-x86_64-unknown-linux-gnu`, `rfx-x86_64-pc-windows-msvc.exe`)
  - Shell and PowerShell installer scripts
  - Auto-generated release notes

## What Gets Released

The GitHub Release will contain:

**Binaries (raw executables, no archives):**
- `rfx-aarch64-apple-darwin` - macOS ARM (Apple Silicon)
- `rfx-aarch64-unknown-linux-gnu` - Linux ARM64
- `rfx-x86_64-apple-darwin` - macOS Intel
- `rfx-x86_64-unknown-linux-gnu` - Linux x64 (glibc)
- `rfx-x86_64-unknown-linux-musl` - Linux x64 (static, no libc)
- `rfx-x86_64-pc-windows-msvc.exe` - Windows x64

**Installers:**
- `reflex-installer.sh` - Shell install script (`curl | sh`)
- `reflex-installer.ps1` - PowerShell install script

## Workflow Configuration

Releases are configured in:
- **`dist-workspace.toml`** - cargo-dist configuration (platforms, installers)
- **`.github/workflows/release.yml`** - GitHub Actions workflow (builds binaries, extracts archives)

**Key settings:**
```toml
# dist-workspace.toml
[dist]
targets = ["aarch64-apple-darwin", "aarch64-unknown-linux-gnu",
           "x86_64-apple-darwin", "x86_64-unknown-linux-gnu",
           "x86_64-unknown-linux-musl", "x86_64-pc-windows-msvc"]
installers = ["shell", "powershell"]
auto-includes = false  # Don't bundle README/CHANGELOG in archives
allow-dirty = ["ci"]   # Allow custom workflow modifications
```

## CHANGELOG.md Format

```markdown
# Changelog

All notable changes to Reflex will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [1.1.0] - 2025-11-03

### Added
- Query timeout support with `--timeout` flag
- HTTP API timeout parameter

### Fixed
- Handle empty files without panicking

## [1.0.0] - 2025-11-01

### Added
- Initial release
- Trigram-based full-text search
- Symbol-aware filtering
- Multi-language support
```
