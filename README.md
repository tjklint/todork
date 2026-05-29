<h1 align="center">todork</h1>

| Default scan | With `--blame` |
|:---:|:---:|
| ![todork scan demo](samples/todork.gif) | ![todork blame demo](samples/todork_blame.gif) |

> Hyper-fast CLI scanner for TODO, FIXME, HACK, and other annotation comments in your codebase.

[![CI](https://github.com/tjklint/todork/actions/workflows/ci.yml/badge.svg)](https://github.com/tjklint/todork/actions/workflows/ci.yml)
[![MIT License](https://img.shields.io/badge/license-MIT-orange.svg)](LICENSE)

## Features

- ⚡ **Blazing fast** — parallel file walking powered by the same engine as [ripgrep](https://github.com/BurntSushi/ripgrep)
- 🔍 Finds `TODO`, `FIXME`, `HACK`, `XXX`, `NOTE`, `OPTIMIZE`, `BUG`, `DEPRECATED`, and custom tags
- 🎨 Colored, human-readable output with file/line/column info
- 📄 JSON and GitHub Actions annotation output formats
- 🚫 Respects `.gitignore` by default
- 💾 No Rust required to install — grab a pre-built binary

## Installation

### One-line install (Linux & macOS)

```sh
curl -fsSL https://raw.githubusercontent.com/tjklint/todork/main/install.sh | sh
```

Set `TODORK_VERSION` to pin a specific release:

```sh
TODORK_VERSION=1.0.0 curl -fsSL https://raw.githubusercontent.com/tjklint/todork/main/install.sh | sh
```

### Pre-built binaries

Download the latest release from the [Releases page](https://github.com/tjklint/todork/releases):

| Platform | Binary |
|----------|--------|
| Linux x86_64 (static) | `todork-*-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 (static) | `todork-*-aarch64-unknown-linux-musl.tar.gz` |
| macOS Intel | `todork-*-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `todork-*-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `todork-*-x86_64-pc-windows-msvc.zip` |

### From source (requires Rust 1.85+)

```sh
cargo install --git https://github.com/tjklint/todork
```

## Usage

```sh
# Scan the current directory
todork .

# Scan specific paths
todork src/ tests/

# Output as JSON
todork . --format json

# Only look for FIXME and BUG
todork . --tags fixme,bug

# Include files ignored by .gitignore
todork . --no-gitignore

# Only scan Python files
todork . --include "*.py"

# GitHub Actions annotation output
todork . --format github-annotations

# Show all flags and options
todork --help

# Upgrade to the latest release
todork upgrade
```

## Upgrading

`todork upgrade` checks GitHub for a newer release, downloads the right binary for your platform, and replaces the current executable in place — no need to re-run the install script.

```sh
todork upgrade
# Checking for updates...
# Update available: 1.0.0 → 1.1.0
# Downloading todork-1.1.0-x86_64-unknown-linux-musl.tar.gz...
# Extracting...
# Upgraded to todork 1.1.0
```

On Windows the replacement may require moving the downloaded binary manually; the command prints the exact `move` command if that happens.

## Output formats

### `text` (default)

```
src/auth.rs:42:5: FIXME: this crashes on empty input
src/utils.rs:17:3: TODO(alice): refactor into smaller functions
src/config.rs:8:1: XXX: hardcoded secret, move to env var

Found 3 annotations across 3 files.
```

### `json`

```json
[
  {
    "file": "src/auth.rs",
    "line": 42,
    "column": 5,
    "tag": "FIXME",
    "author": null,
    "message": "this crashes on empty input"
  }
]
```

### `github-annotations`

```
::error file=src/auth.rs,line=42,col=5,title=FIXME::this crashes on empty input
::warning file=src/utils.rs,line=17,col=3,title=TODO::refactor into smaller functions
```

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Annotations found |
| `1` | No annotations found |
| `2` | Fatal error (bad arguments, I/O failure) |

Use `--exit-zero` to always exit `0` (useful for non-blocking CI reporting).

## Default tags

| Tag | Severity |
|-----|----------|
| `FIXME` | Error |
| `BUG` | Error |
| `XXX` | Error |
| `TODO` | Warning |
| `HACK` | Warning |
| `DEPRECATED` | Warning |
| `NOTE` | Info |
| `OPTIMIZE` | Info |

## License

[MIT](LICENSE) © TJ Klint
