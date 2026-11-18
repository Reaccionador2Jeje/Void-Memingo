#!/usr/bin/env bash
set -euo pipefail

IMAGE="${1:?Usage: test-void-boot-media-uefi.sh <void-boot-media.img> [log-file]}"
LOG_FILE="${2:-${TMPDIR:-/tmp}/void-boot-media-uefi.log}"
[[ -s "$IMAGE" ]] || { echo "Media image is missing or empty: $IMAGE" >&2; exit 1; }

for tool in qemu-system-x86_64 timeout cp grep; do
  command -v "$tool" >/dev/null 2>&1 || { echo "Required tool not found: $tool" >&2; exit 1; }
done

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
  echo "Readable OVMF_CODE and OVMF_VARS firmware files are required." >&2
  exit 1
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
VARS_COPY="$WORK_DIR/OVMF_VARS.fd"
cp "$OVMF_VARS" "$VARS_COPY"
mkdir -p "$(dirname "$LOG_FILE")"

set +e
timeout 35s qemu-system-x86_64 \
  -machine q35,accel=tcg \
  -cpu max \
  -m 1024 \
  -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE" \
  -drive "if=pflash,format=raw,file=$VARS_COPY" \
  -drive "file=$IMAGE,format=raw,if=virtio" \
  -boot order=c \
  -display none \
  -serial "file:$LOG_FILE" \
  -monitor none \
  -no-reboot \
  -no-shutdown \
  -net none
status=$?
set -e

if [[ "$status" -ne 124 && "$status" -ne 0 ]]; then
  echo "QEMU exited unexpectedly with status $status." >&2
  cat "$LOG_FILE" >&2 || true
  exit 1
fi
if ! grep -q "VOID_BOOT_MEDIA_READY" "$LOG_FILE"; then
  echo "UEFI firmware did not reach the Void Boot GRUB marker." >&2
  cat "$LOG_FILE" >&2 || true
  exit 1
fi

echo "UEFI media image booted through OVMF and reached the Void Boot GRUB marker."
