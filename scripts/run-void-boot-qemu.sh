#!/usr/bin/env bash
set -euo pipefail

IMAGE="${1:?Uso: scripts/run-void-boot-qemu.sh <void-boot.img>}"
[[ -f "$IMAGE" && -s "$IMAGE" ]] || { echo "La imagen virtual no existe o está vacía: $IMAGE" >&2; exit 1; }

for tool in qemu-system-x86_64 mktemp cp; do
  command -v "$tool" >/dev/null 2>&1 || { echo "Falta la herramienta requerida: $tool" >&2; exit 1; }
done

OVMF_CODE="${OVMF_CODE:-}"
OVMF_VARS="${OVMF_VARS:-}"
if [[ -z "$OVMF_CODE" || -z "$OVMF_VARS" ]]; then
  for pair in \
    "/usr/share/OVMF/OVMF_CODE_4M.fd:/usr/share/OVMF/OVMF_VARS_4M.fd" \
    "/usr/share/OVMF/OVMF_CODE.fd:/usr/share/OVMF/OVMF_VARS.fd" \
    "/usr/share/edk2/ovmf/OVMF_CODE_4M.fd:/usr/share/edk2/ovmf/OVMF_VARS_4M.fd" \
    "/usr/share/edk2/ovmf/OVMF_CODE.fd:/usr/share/edk2/ovmf/OVMF_VARS.fd"; do
    IFS=: read -r code vars <<< "$pair"
    if [[ -r "$code" && -r "$vars" ]]; then
      OVMF_CODE="$code"; OVMF_VARS="$vars"; break
    fi
  done
fi
[[ -r "$OVMF_CODE" && -r "$OVMF_VARS" ]] || {
  echo "No se encontraron OVMF_CODE/OVMF_VARS. Instala ovmf o define ambas variables." >&2
  exit 1
}

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
cp "$OVMF_VARS" "$WORK_DIR/OVMF_VARS.fd"

echo "Iniciando Void Boot en QEMU/UEFI."
echo "Disco invitado: solo lectura lógica mediante snapshot temporal; no se modifica el archivo original."
echo "Red deshabilitada. Cierra la ventana de QEMU para terminar."
qemu-system-x86_64 \
  -machine q35,accel=tcg \
  -cpu max \
  -m "${VOID_BOOT_QEMU_MEMORY:-2048}" \
  -drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE" \
  -drive "if=pflash,format=raw,file=$WORK_DIR/OVMF_VARS.fd" \
  -drive "file=$IMAGE,format=raw,if=virtio,snapshot=on" \
  -boot order=c \
  -net none \
  -no-reboot
