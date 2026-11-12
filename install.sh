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

command -v python3 >/dev/null 2>&1 || fail "Python 3 is required to download and verify Void Memingo."

if [ -w /usr/local/bin ]; then
  INSTALL_DIR="/usr/local/bin"
elif command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
  INSTALL_DIR="/usr/local/bin"
else
  INSTALL_DIR="${HOME}/.local/bin"
  mkdir -p "$INSTALL_DIR"
fi

BASE_URL="https://github.com/$REPO/releases/latest/download"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT HUP INT TERM

say "Downloading Void Memingo for $OS/$ARCH (using Python, no curl/wget)..."
python3 - "$BASE_URL" "$TMP_DIR/void" "$TMP_DIR/void.sha256" <<'PY'
import hashlib
import sys
import urllib.error
import urllib.request

base_url, binary_path, checksum_path = sys.argv[1:4]
files = {
    binary_path: base_url + "/void-linux-x86_64",
    checksum_path: base_url + "/void-linux-x86_64.sha256",
}

for destination, url in files.items():
    request = urllib.request.Request(
        url,
        headers={"User-Agent": "Void-Memingo-Installer/0.2"},
    )
    try:
        with urllib.request.urlopen(request, timeout=60) as response:
            data = response.read()
    except urllib.error.HTTPError as error:
        if error.code == 404:
            print(
                "Installer error: GitHub has no published release asset yet. "
                "A maintainer must publish the v0.2.0 release with "
                "void-linux-x86_64 and void-linux-x86_64.sha256.",
                file=sys.stderr,
            )
        else:
            print(f"Installer error: download failed (HTTP {error.code}).", file=sys.stderr)
        sys.exit(1)
    except Exception as error:
        print(f"Installer error: download failed: {error}", file=sys.stderr)
        sys.exit(1)
    with open(destination, "wb") as output:
        output.write(data)

with open(checksum_path, encoding="utf-8") as checksum_file:
    expected = checksum_file.read().split()[0].lower()

digest = hashlib.sha256()
with open(binary_path, "rb") as binary_file:
    for chunk in iter(lambda: binary_file.read(1024 * 1024), b""):
        digest.update(chunk)
actual = digest.hexdigest()

if expected != actual:
    print("Installer error: SHA-256 checksum verification failed.", file=sys.stderr)
    sys.exit(1)
PY

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
