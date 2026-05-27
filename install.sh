#!/bin/sh
# todork installer
# Usage: curl -fsSL https://raw.githubusercontent.com/tjklint/todork/main/install.sh | sh
# Options (env vars):
#   TODORK_VERSION   - version to install (default: latest)
#   TODORK_INSTALL_DIR - where to put the binary (default: ~/.local/bin)

set -e

REPO="tjklint/todork"
BINARY="todork"

# ── helpers ──────────────────────────────────────────────────────────────────

say()  { printf "\033[1;32m[todork]\033[0m %s\n" "$*"; }
warn() { printf "\033[1;33m[todork]\033[0m %s\n" "$*" >&2; }
err()  { printf "\033[1;31m[todork]\033[0m ERROR: %s\n" "$*" >&2; exit 1; }

need() {
    command -v "$1" >/dev/null 2>&1 || err "'$1' is required but not found in PATH"
}

# ── detect platform ───────────────────────────────────────────────────────────

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
    Linux*)
        case "$ARCH" in
            x86_64)  TARGET="x86_64-unknown-linux-musl"  ;;
            aarch64|arm64) TARGET="aarch64-unknown-linux-musl" ;;
            *) err "Unsupported Linux architecture: $ARCH. Download manually from https://github.com/$REPO/releases" ;;
        esac
        EXT="tar.gz"
        ;;
    Darwin*)
        case "$ARCH" in
            x86_64)  TARGET="x86_64-apple-darwin"  ;;
            arm64)   TARGET="aarch64-apple-darwin"  ;;
            *) err "Unsupported macOS architecture: $ARCH" ;;
        esac
        EXT="tar.gz"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        warn "Windows detected. Please download the .zip from https://github.com/$REPO/releases"
        warn "Or use: winget install tjklint.todork  (once published)"
        exit 0
        ;;
    *)
        err "Unsupported OS: $OS. Download manually from https://github.com/$REPO/releases"
        ;;
esac

# ── resolve version ───────────────────────────────────────────────────────────

need curl
need tar

if [ -z "$TODORK_VERSION" ]; then
    say "Fetching latest release..."
    TODORK_VERSION="$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name"' \
        | sed 's/.*"tag_name": *"v\{0,1\}\([^"]*\)".*/\1/')"
    [ -n "$TODORK_VERSION" ] || err "Could not determine latest version. Set TODORK_VERSION manually."
fi

say "Installing todork v${TODORK_VERSION} (${TARGET})..."

# ── resolve install dir ────────────────────────────────────────────────────────

if [ -n "$TODORK_INSTALL_DIR" ]; then
    INSTALL_DIR="$TODORK_INSTALL_DIR"
elif [ -d "$HOME/.local/bin" ] || echo "$PATH" | grep -q "$HOME/.local/bin"; then
    INSTALL_DIR="$HOME/.local/bin"
elif echo "$PATH" | grep -q "/usr/local/bin"; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
fi

mkdir -p "$INSTALL_DIR"

# ── download & verify ─────────────────────────────────────────────────────────

BASE_URL="https://github.com/$REPO/releases/download/v${TODORK_VERSION}"
ARCHIVE="${BINARY}-${TODORK_VERSION}-${TARGET}.${EXT}"
CHECKSUM_FILE="${ARCHIVE}.sha256"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

say "Downloading ${ARCHIVE}..."
curl -fsSL "${BASE_URL}/${ARCHIVE}"          -o "${TMP_DIR}/${ARCHIVE}"
curl -fsSL "${BASE_URL}/${CHECKSUM_FILE}"    -o "${TMP_DIR}/${CHECKSUM_FILE}"

say "Verifying checksum..."
cd "$TMP_DIR"
if command -v sha256sum >/dev/null 2>&1; then
    sha256sum --check "$CHECKSUM_FILE" || err "Checksum verification failed!"
elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 --check "$CHECKSUM_FILE" || err "Checksum verification failed!"
else
    warn "No sha256sum or shasum found — skipping checksum verification"
fi
cd - >/dev/null

# ── extract & install ─────────────────────────────────────────────────────────

say "Extracting..."
tar xzf "${TMP_DIR}/${ARCHIVE}" -C "$TMP_DIR"
chmod +x "${TMP_DIR}/${BINARY}"

say "Installing to ${INSTALL_DIR}/${BINARY}..."
mv "${TMP_DIR}/${BINARY}" "${INSTALL_DIR}/${BINARY}"

# ── verify ───────────────────────────────────────────────────────────────────

if "${INSTALL_DIR}/${BINARY}" --version >/dev/null 2>&1; then
    say "✓ todork $(${INSTALL_DIR}/${BINARY} --version) installed successfully!"
else
    say "✓ todork installed to ${INSTALL_DIR}/${BINARY}"
fi

# ── PATH reminder ─────────────────────────────────────────────────────────────

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        warn "${INSTALL_DIR} is not in your PATH."
        warn "Add this to your shell profile:"
        warn "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        ;;
esac
