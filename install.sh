#!/bin/sh
set -eu

REPO="Reaccionador2Jeje/Void-Memingo"
BINARY="void"
INSTALL_DIR=""

say() { printf '%s\n' "Void Memingo Installer: $*"; }
fail() { printf '%s\n' "Installer error: $*" >&2; exit 1; }

case "$(uname -s)" in
  Linux) OS="linux" ;;
  *) fail "This installer currently supports Linux only." ;;
esac

case "$(uname -m)" in
  x86_64|amd64) ARCH="x86_64" ;;
  *) fail "Unsupported architecture: $(uname -m). Currently supported: x86_64." ;;
esac

if [ -w /usr/local/bin ]; then
  INSTALL_DIR="/usr/local/bin"
elif command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
  INSTALL_DIR="/usr/local/bin"
else
  INSTALL_DIR="${HOME}/.local/bin"
  mkdir -p "$INSTALL_DIR"
fi

command -v curl >/dev/null 2>&1 || fail "curl is required."
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required."

BASE_URL="https://github.com/$REPO/releases/latest/download"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT HUP INT TERM

say "Downloading Void Memingo for $OS/$ARCH..."
curl -fL --retry 3 "$BASE_URL/void-$OS-$ARCH" -o "$TMP_DIR/void"
curl -fL --retry 3 "$BASE_URL/void-$OS-$ARCH.sha256" -o "$TMP_DIR/void.sha256"

EXPECTED="$(awk '{print $1}' "$TMP_DIR/void.sha256")"
ACTUAL="$(sha256sum "$TMP_DIR/void" | awk '{print $1}')"
[ "$EXPECTED" = "$ACTUAL" ] || fail "Checksum verification failed."

chmod 755 "$TMP_DIR/void"

if [ "$INSTALL_DIR" = "/usr/local/bin" ] && [ ! -w "$INSTALL_DIR" ]; then
  sudo install -m 755 "$TMP_DIR/void" "$INSTALL_DIR/$BINARY"
else
  install -m 755 "$TMP_DIR/void" "$INSTALL_DIR/$BINARY"
fi

say "Installed successfully: $INSTALL_DIR/$BINARY"
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *) say "Add this directory to PATH: $INSTALL_DIR" ;;
esac
say "Run: void --help"
