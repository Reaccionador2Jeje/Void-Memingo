#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE_FILE="${1:-$ROOT_DIR/apps/void-boot/profiles/example-profiles.toml}"
OUTPUT_ZIP="${2:-$ROOT_DIR/target/void-boot-installer.zip}"
STAGING_DIR="$(mktemp -d)"
PACKAGE_DIR="$STAGING_DIR/Void-Boot"

cleanup() { rm -rf "$STAGING_DIR"; }
trap cleanup EXIT

command -v zip >/dev/null 2>&1 || {
  echo "zip is required to create the Void Boot installer archive." >&2
  exit 1
}
test -f "$PROFILE_FILE" || {
  echo "Profile catalog not found: $PROFILE_FILE" >&2
  exit 1
}

mkdir -p "$PACKAGE_DIR/payload/boot/void-boot/profiles" "$(dirname "$OUTPUT_ZIP")"

cargo run --manifest-path "$ROOT_DIR/apps/void-boot/Cargo.toml" -- \
  render-menu "$PROFILE_FILE" "$PACKAGE_DIR/payload/boot/grub"

cp "$ROOT_DIR"/apps/void-boot/profiles/*.toml "$PACKAGE_DIR/payload/boot/void-boot/profiles/"
cp "$ROOT_DIR/apps/void-boot/README.md" "$PACKAGE_DIR/README-VOID-BOOT.md"
cp "$ROOT_DIR/scripts/install-void-boot.sh" "$PACKAGE_DIR/install.sh"
cp "$ROOT_DIR/scripts/install-void-boot.ps1" "$PACKAGE_DIR/install.ps1"
cp "$ROOT_DIR/apps/void-boot/installers/web/index.html" "$PACKAGE_DIR/web-installer.html"

cat > "$PACKAGE_DIR/INSTALLATION-STATUS.txt" <<'STATUS'
Void Boot package: menu and profile deployment preview.

This package copies Void Boot GRUB menu configuration, theme and profile
catalogues into a selected directory. It does NOT install GRUB, create EFI
boot files, partition or format a drive, or make a USB bootable.

A future BIOS/UEFI media installer will be delivered only after dedicated
virtual-disk tests and explicit destructive-operation safeguards.
STATUS

(
  cd "$STAGING_DIR"
  zip -qr "$OUTPUT_ZIP" Void-Boot
)

test -s "$OUTPUT_ZIP"
echo "Created Void Boot installer package: $OUTPUT_ZIP"
echo "Package contents are a menu/profile deployment preview, not a bootable USB installer."
