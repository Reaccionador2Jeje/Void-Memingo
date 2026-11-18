#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PAYLOAD_DIR="$SCRIPT_DIR/payload"

if [[ ! -d "$PAYLOAD_DIR/boot/grub" ]]; then
  echo "Void Boot payload was not found. Extract the complete ZIP before running install.sh." >&2
  exit 1
fi

TARGET_DIR="${1:-}"
if [[ -z "$TARGET_DIR" ]]; then
  read -r -p "Destination directory (existing mounted folder or USB mount point): " TARGET_DIR
fi
if [[ -z "$TARGET_DIR" ]]; then
  echo "No destination selected." >&2
  exit 1
fi

case "$TARGET_DIR" in\n  /*) ;;\n  *) TARGET_DIR="$PWD/$TARGET_DIR" ;;\nesac\nmkdir -p "$TARGET_DIR"\nTARGET_DIR="$(cd "$TARGET_DIR" && pwd -P)"
case "$TARGET_DIR" in
  /|/boot|/etc|/usr|/bin|/sbin|/lib|/lib64)
    echo "Refusing to deploy Void Boot into a protected system directory: $TARGET_DIR" >&2
    exit 1
    ;;
esac

echo "This copies menu and profile files into: $TARGET_DIR"
echo "It does not install GRUB or make the destination bootable."
read -r -p "Continue? [y/N] " CONFIRM
case "$CONFIRM" in
  y|Y|yes|YES) ;;
  *) echo "Cancelled."; exit 0 ;;
esac

cp -R "$PAYLOAD_DIR/." "$TARGET_DIR/"
printf '%s\n' "Void Boot menu/profile files deployed." > "$TARGET_DIR/VOID-BOOT-INSTALL.txt"
echo "Void Boot files copied successfully."
