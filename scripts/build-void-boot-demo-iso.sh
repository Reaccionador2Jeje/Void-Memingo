#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE_FILE="${1:-$ROOT_DIR/apps/void-boot/profiles/example-profiles.toml}"
OUTPUT_ISO="${2:-$ROOT_DIR/target/void-boot-demo.iso}"
STAGING_DIR="$(mktemp -d)"
trap 'rm -rf "$STAGING_DIR"' EXIT

if ! command -v grub-mkrescue >/dev/null 2>&1; then
  echo "grub-mkrescue is required (install GRUB BIOS/UEFI tools and xorriso)." >&2
  exit 1
fi

mkdir -p "$STAGING_DIR/iso" "$(dirname "$OUTPUT_ISO")"
cargo run --manifest-path "$ROOT_DIR/apps/void-boot/Cargo.toml" -- \
  render-menu "$PROFILE_FILE" "$STAGING_DIR/boot/grub"

if command -v grub-script-check >/dev/null 2>&1; then
  grub-script-check "$STAGING_DIR/boot/grub/grub.cfg"
fi

grub-mkrescue -o "$OUTPUT_ISO" "$STAGING_DIR"
test -s "$OUTPUT_ISO"
echo "Created bootable Void Boot menu ISO: $OUTPUT_ISO"
echo "This is a menu prototype. The example profile ISO is not bundled."
echo "No physical drive was modified."
