#!/usr/bin/env bash
# RNF Installer
# Usage: curl -sSL https://raw.githubusercontent.com/risqinf/rnf/main/install.sh | bash

set -e

REPO="risqinf/rnf"
INSTALL_DIR="${RNF_INSTALL_DIR:-/usr/local/bin}"
TMP_DIR=$(mktemp -d)

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()    { echo -e "${CYAN}→${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
error()   { echo -e "${RED}✗ Error:${NC} $1" >&2; exit 1; }

echo -e "${BOLD}${CYAN}"
echo "  ██████╗ ███╗   ██╗███████╗"
echo "  ██╔══██╗████╗  ██║██╔════╝"
echo "  ██████╔╝██╔██╗ ██║█████╗  "
echo "  ██╔══██╗██║╚██╗██║██╔══╝  "
echo "  ██║  ██║██║ ╚████║██║     "
echo "  ╚═╝  ╚═╝╚═╝  ╚═══╝╚═╝     "
echo -e "${NC}"
echo -e "${BOLD}RNF Language Installer${NC}"
echo ""

# Detect OS and arch
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)
    case "$ARCH" in
      x86_64)  ASSET="rnf-linux-x86_64-musl.tar.gz" ;;
      aarch64) ASSET="rnf-linux-aarch64-musl.tar.gz" ;;
      *) error "Unsupported architecture: $ARCH" ;;
    esac
    ;;
  darwin)
    case "$ARCH" in
      x86_64) ASSET="rnf-macos-x86_64.tar.gz" ;;
      arm64)  ASSET="rnf-macos-aarch64.tar.gz" ;;
      *) error "Unsupported architecture: $ARCH" ;;
    esac
    ;;
  *) error "Unsupported OS: $OS" ;;
esac

info "Detected: $OS/$ARCH"
info "Package:  $ASSET"

# Get latest release
LATEST=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
if [ -z "$LATEST" ]; then
  error "Could not fetch latest release. Check your internet connection."
fi
info "Latest version: $LATEST"

# Download
URL="https://github.com/${REPO}/releases/download/${LATEST}/${ASSET}"
info "Downloading $URL"
curl -sSL "$URL" -o "$TMP_DIR/$ASSET" || error "Download failed"

# Extract
info "Extracting…"
tar xzf "$TMP_DIR/$ASSET" -C "$TMP_DIR"

# Install
BINARY=$(ls "$TMP_DIR"/rnf-* 2>/dev/null | head -1)
if [ -z "$BINARY" ]; then
  error "Binary not found in archive"
fi

info "Installing to $INSTALL_DIR/rnf"
if [ -w "$INSTALL_DIR" ]; then
  cp "$BINARY" "$INSTALL_DIR/rnf"
  chmod +x "$INSTALL_DIR/rnf"
else
  sudo cp "$BINARY" "$INSTALL_DIR/rnf"
  sudo chmod +x "$INSTALL_DIR/rnf"
fi

# Cleanup
rm -rf "$TMP_DIR"

success "RNF installed successfully!"
echo ""
echo -e "  ${BOLD}Version:${NC} $($INSTALL_DIR/rnf version 2>&1 | head -1 || echo $LATEST)"
echo -e "  ${BOLD}Binary:${NC}  $INSTALL_DIR/rnf"
echo ""
echo -e "  ${CYAN}Quick start:${NC}"
echo "    rnf init myproject"
echo "    cd myproject"
echo "    rnf --run src/main.rnf"
echo "    rnf --release src/main.rnf"
echo ""
echo -e "  ${CYAN}Docs:${NC} https://github.com/risqinf/rnf"
