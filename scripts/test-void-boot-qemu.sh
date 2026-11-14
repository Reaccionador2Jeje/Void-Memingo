#!/usr/bin/env bash
set -euo pipefail

ISO_PATH="${1:?Usage: test-void-boot-qemu.sh <serial-test.iso>}"
LOG_DIR="${2:-$(dirname "$ISO_PATH")/void-boot-qemu-logs}"
mkdir -p "$LOG_DIR"

if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
  echo "qemu-system-x86_64 is required." >&2
  exit 1
fi

OVMF_CODE="${OVMF_CODE:-}"
OVMF_VARS="${OVMF_VARS:-}"

# Prefer the standard firmware pair; 4M images are a fallback when both files exist.
if [[ -z "$OVMF_CODE" || -z "$OVMF_VARS" ]]; then
  for pair in \
    "/usr/share/OVMF/OVMF_CODE.fd:/usr/share/OVMF/OVMF_VARS.fd" \
    "/usr/share/OVMF/OVMF_CODE_4M.fd:/usr/share/OVMF/OVMF_VARS_4M.fd" \
    "/usr/share/edk2/OVMF_CODE.fd:/usr/share/edk2/OVMF_VARS.fd" \
    "/usr/share/edk2/OVMF_CODE_4M.fd:/usr/share/edk2/OVMF_VARS_4M.fd"; do
    IFS=: read -r candidate_code candidate_vars <<< "$pair"
    if [[ -z "$OVMF_CODE" && -r "$candidate_code" && -z "$OVMF_VARS" && -r "$candidate_vars" ]]; then
      OVMF_CODE="$candidate_code"
      OVMF_VARS="$candidate_vars"
      break
    fi
  done
fi

if [[ -z "$OVMF_CODE" || ! -r "$OVMF_CODE" || -z "$OVMF_VARS" || ! -r "$OVMF_VARS" ]]; then
  echo "A readable OVMF_CODE and matching OVMF_VARS firmware pair was not found." >&2
  exit 1
fi

run_firmware_test() {
  local mode="$1"
  shift
  local log_file="$LOG_DIR/$mode.log"
  rm -f "$log_file"
  if [[ "$mode" == "uefi" ]]; then
    cp "$OVMF_VARS" "$LOG_DIR/OVMF_VARS-uefi.fd"
  fi

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
run_firmware_test uefi \
  -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE" \
  -drive "if=pflash,format=raw,file=$LOG_DIR/OVMF_VARS-uefi.fd"
