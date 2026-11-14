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

OVMF_CODE="${OVMF_CODE:-}"
OVMF_VARS="${OVMF_VARS:-}"
if [[ -z "$OVMF_CODE" || -z "$OVMF_VARS" ]]; then
  for pair in \
    "/usr/share/OVMF/OVMF_CODE.fd:/usr/share/OVMF/OVMF_VARS.fd" \
    "/usr/share/OVMF/OVMF_CODE_4M.fd:/usr/share/OVMF/OVMF_VARS_4M.fd" \
    "/usr/share/edk2/ovmf/OVMF_CODE.fd:/usr/share/edk2/ovmf/OVMF_VARS.fd" \
    "/usr/share/edk2/ovmf/OVMF_CODE_4M.fd:/usr/share/edk2/ovmf/OVMF_VARS_4M.fd"; do
    IFS=: read -r candidate_code candidate_vars <<< "$pair"
    if [[ -r "$candidate_code" && -r "$candidate_vars" ]]; then
      OVMF_CODE="$candidate_code"
      OVMF_VARS="$candidate_vars"
      break
    fi
  done
fi
if [[ -z "$OVMF_CODE" || ! -r "$OVMF_CODE" || -z "$OVMF_VARS" || ! -r "$OVMF_VARS" ]]; then
  echo "A readable OVMF_CODE and matching OVMF_VARS pair is required for BIOS+UEFI testing." >&2
  exit 1
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
ISO_DISK="$WORK_DIR/debian-live-media.img"

# This is a disposable virtual FAT disk file, never a physical drive.
truncate -s 2200M "$ISO_DISK"
mkfs.vfat "$ISO_DISK" >/dev/null
mmd -i "$ISO_DISK" ::/iso
mcopy -i "$ISO_DISK" "$DEBIAN_ISO" ::/iso/debian-live-amd64.iso

run_live_test() {
  local mode="$1"
  local log_file="$LOG_DIR/debian-live-$mode.log"
  local -a firmware_args=()
  if [[ "$mode" == "uefi" ]]; then
    local vars_copy="$WORK_DIR/OVMF_VARS.fd"
    cp "$OVMF_VARS" "$vars_copy"
    firmware_args+=(
      -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE"
      -drive "if=pflash,format=raw,file=$vars_copy"
    )
  fi

  set +e
  timeout 240s qemu-system-x86_64 \
    -machine q35,accel=tcg \
    -cpu max \
    -m 3072 \
    -drive "file=$MENU_ISO,media=cdrom,readonly=on,if=ide,index=0" \
    -drive "file=$ISO_DISK,format=raw,if=ide,index=1" \
    -boot d \
    -display none \
    -serial "file:$log_file" \
    -monitor none \
    -no-reboot \
    -no-shutdown \
    -net none \
    "${firmware_args[@]}"
  local status=$?
  set -e

  # A timeout is expected after Linux reaches its live console.
  if [[ "$status" -ne 124 && "$status" -ne 0 ]]; then
    echo "$mode QEMU exited unexpectedly with status $status." >&2
    cat "$log_file" >&2 || true
    return 1
  fi
  if ! grep -Eiq 'Debian GNU/Linux|Debian Live|login:' "$log_file"; then
    echo "Debian Live did not reach a recognizable $mode userspace console." >&2
    cat "$log_file" >&2 || true
    return 1
  fi
  echo "Debian Live reached its userspace console through Void Boot ($mode)."
}

run_live_test bios
run_live_test uefi
