#!/usr/bin/env bash
set -euo pipefail

MENU_ISO="${1:?Usage: test-debian-live-qemu.sh <void-boot-menu.iso> <debian-live.iso> [log-dir]}"
DEBIAN_ISO="${2:?Missing Debian Live ISO path}"
LOG_DIR="${3:-$(dirname "$MENU_ISO")/debian-live-qemu-logs}"
mkdir -p "$LOG_DIR"

for tool in qemu-system-x86_64 timeout truncate mkfs.vfat mmd mcopy; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "Required tool not found: $tool" >&2
    exit 1
  }
done

[[ -s "$MENU_ISO" ]] || { echo "Void Boot menu ISO is missing or empty." >&2; exit 1; }
[[ -s "$DEBIAN_ISO" ]] || { echo "Debian Live ISO is missing or empty." >&2; exit 1; }

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
ISO_DISK="$WORK_DIR/debian-live-media.img"
LOG_FILE="$LOG_DIR/debian-live-boot.log"

# A disposable virtual FAT disk carries the Debian ISO. No physical device is used.
truncate -s 2200M "$ISO_DISK"
mkfs.vfat "$ISO_DISK" >/dev/null
mmd -i "$ISO_DISK" ::/iso
mcopy -i "$ISO_DISK" "$DEBIAN_ISO" ::/iso/debian-live-amd64.iso

set +e
timeout 240s qemu-system-x86_64 \
  -machine q35,accel=tcg \
  -cpu max \
  -m 3072 \
  -drive "file=$MENU_ISO,media=cdrom,readonly=on,if=ide,index=0" \
  -drive "file=$ISO_DISK,format=raw,if=ide,index=1" \
  -boot d \
  -display none \
  -serial "file:$LOG_FILE" \
  -monitor none \
  -no-reboot \
  -no-shutdown \
  -net none
STATUS=$?
set -e

# A timeout is expected after Linux has booted and remains at its live console.
if [[ "$STATUS" -ne 124 && "$STATUS" -ne 0 ]]; then
  echo "QEMU exited unexpectedly with status $STATUS." >&2
  cat "$LOG_FILE" >&2 || true
  exit 1
fi

if ! grep -Eiq 'Debian GNU/Linux|Debian Live|login:' "$LOG_FILE"; then
  echo "Debian Live did not reach a recognizable userspace console." >&2
  cat "$LOG_FILE" >&2 || true
  exit 1
fi

echo "Debian Live reached its userspace console through Void Boot."
