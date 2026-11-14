#!/usr/bin/env bash
set -euo pipefail

ISO_PATH="${1:?Usage: test-void-boot-qemu.sh <serial-test.iso>}"
LOG_DIR="${2:-$(dirname "$ISO_PATH")/void-boot-qemu-logs}"
mkdir -p "$LOG_DIR"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "qemu-system-x86_64 is required." >&2
  exit 1
fi

OVMF_CODE="${OVMF_CODE:-$(find /usr/share/OVMF /usr/share/edk2 -type f \( -name 'OVMF_CODE.fd' -o -name 'OVMF_CODE_4M.fd' \) -print -quit 2>/dev/null || true)}"
if [[ -z "$OVMF_CODE" ]]; then
  echo "OVMF UEFI firmware was not found. Install the ovmf package." >&2
  exit 1
fi

run_firmware_test() {
  local mode="$1"
  shift
  local log_file="$LOG_DIR/$mode.log"
  rm -f "$log_file"

  set +e
  timeout 18s qemu-system-x86_64 \
    -machine q35,accel=tcg \
    -m 512 \
    -cdrom "$ISO_PATH" \
    -boot d \
    -display none \
    -serial "file:$log_file" \
    -monitor none \
    -no-reboot \
    -no-shutdown \
    "$@"
  local status=$?
  set -e

  if [[ "$status" -ne 124 && "$status" -ne 0 ]]; then
    echo "$mode QEMU exited unexpectedly with status $status" >&2
    cat "$log_file" >&2 || true
    return 1
  fi

  if ! grep -q "VOID_BOOT_MENU_READY" "$log_file"; then
    echo "$mode did not reach the Void Boot GRUB menu." >&2
    cat "$log_file" >&2 || true
    return 1
  fi
  if ! grep -q "Debian Live (example profile)" "$log_file"; then
    echo "$mode did not render the example profile entry." >&2
    cat "$log_file" >&2 || true
    return 1
  fi

  echo "$mode booted the Void Boot menu successfully."
}

run_firmware_test bios
run_firmware_test uefi -bios "$OVMF_CODE"
