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
cargo run --quiet --manifest-path "$ROOT_DIR/apps/void-boot/Cargo.toml" --features internal-cli --bin void-boot-internal -- \
  render-menu "$PROFILE_FILE" "$STAGING_DIR/boot/grub"

if [[ "${VOID_BOOT_SERIAL_TEST:-0}" == "1" ]]; then
  cat >> "$STAGING_DIR/boot/grub/grub.cfg" <<'GRUB_SERIAL_TEST'
insmod serial
serial --unit=0 --speed=115200
terminal_input serial
terminal_output serial
echo "VOID_BOOT_MENU_READY"
GRUB_SERIAL_TEST
fi

if command -v grub-script-check >/dev/null 2>&1; then
  grub-script-check "$STAGING_DIR/boot/grub/grub.cfg"
fi

FONT_PATH="$(find /usr/share/grub /usr/share/grub2 -type f -name unicode.pf2 -print -quit 2>/dev/null || true)"
if [[ -z "$FONT_PATH" ]]; then
  echo "GRUB unicode.pf2 font was not found; cannot guarantee graphical menu rendering." >&2
  exit 1
fi
mkdir -p "$STAGING_DIR/boot/grub/fonts"
cp "$FONT_PATH" "$STAGING_DIR/boot/grub/fonts/unicode.pf2"

grub-mkrescue -o "$OUTPUT_ISO" "$STAGING_DIR"
test -s "$OUTPUT_ISO"
echo "Created bootable Void Boot menu ISO: $OUTPUT_ISO"
echo "This is a menu prototype. The example profile ISO is not bundled."
echo "No physical drive was modified."
