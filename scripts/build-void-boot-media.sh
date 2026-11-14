#!/usr/bin/env bash
set -euo pipefail

# Build a UEFI-bootable raw disk image. This script only writes to a regular
# output file; it never opens, discovers, formats or writes a physical device.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PROFILE_FILE="${1:?Usage: build-void-boot-media.sh <profiles.toml> <source.iso|iso-folder> <output.img> [iso-relative-path]}"
SOURCE_ISO="${2:?Missing source ISO}"
OUTPUT_IMAGE="${3:?Missing output image path}"
ISO_REL_PATH="${4:-}"
ISO_LIBRARY_MODE=0
if [[ -d "$SOURCE_ISO" ]]; then
  ISO_LIBRARY_MODE=1
fi

[[ -f "$PROFILE_FILE" ]] || { echo "Profile catalog not found: $PROFILE_FILE" >&2; exit 1; }
if (( ISO_LIBRARY_MODE == 0 )); then
  [[ -s "$SOURCE_ISO" ]] || { echo "Source ISO is missing or empty: $SOURCE_ISO" >&2; exit 1; }
  if [[ -z "$ISO_REL_PATH" ]]; then
    ISO_REL_PATH="$(sed -n 's/^[[:space:]]*iso[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$PROFILE_FILE" | head -n1)"
  fi
  ISO_PATHS=("$ISO_REL_PATH")
else
  mapfile -t ISO_PATHS < <(sed -n 's/^[[:space:]]*iso[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$PROFILE_FILE" | sort -u)
  (( ${#ISO_PATHS[@]} > 0 )) || { echo "No ISO paths found in profile catalog." >&2; exit 1; }
fi

for ISO_PATH in "${ISO_PATHS[@]}"; do
  if [[ ! "$ISO_PATH" =~ ^[A-Za-z0-9._/-]+\.iso$ ]] || [[ "$ISO_PATH" == /* ]] || [[ "$ISO_PATH" == *..* ]]; then
    echo "Unsafe ISO destination path: $ISO_PATH" >&2
    exit 1
  fi
  if (( ISO_LIBRARY_MODE == 1 )); then
    ISO_SOURCE="$SOURCE_ISO/$(basename "$ISO_PATH")"
    [[ -f "$ISO_SOURCE" && ! -L "$ISO_SOURCE" && -s "$ISO_SOURCE" ]] || {
      echo "Missing, empty or symlinked library ISO: $ISO_SOURCE" >&2
      exit 1
    }
  fi
done

if (( ISO_LIBRARY_MODE == 1 )); then
  declare -A ISO_BASENAMES=()
  for ISO_PATH in "${ISO_PATHS[@]}"; do
    ISO_BASE="$(basename "$ISO_PATH")"
    if [[ -n "${ISO_BASENAMES[$ISO_BASE]:-}" ]]; then
      echo "Two profile paths use the same ISO filename: $ISO_BASE" >&2
      exit 1
    fi
    ISO_BASENAMES[$ISO_BASE]=1
  done
fi

for tool in cargo grub-mkstandalone sgdisk mformat mmd mcopy mdir mktemp truncate; do
  command -v "$tool" >/dev/null 2>&1 || { echo "Required tool not found: $tool" >&2; exit 1; }
done

mkdir -p "$(dirname "$OUTPUT_IMAGE")"
OUTPUT_IMAGE="$(cd "$(dirname "$OUTPUT_IMAGE")" && pwd)/$(basename "$OUTPUT_IMAGE")"
if [[ -e "$OUTPUT_IMAGE" ]]; then
  echo "Refusing to overwrite an existing output file: $OUTPUT_IMAGE" >&2
  exit 1
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT
MENU_DIR="$WORK_DIR/menu"
ESP_DIR="$WORK_DIR/esp"
mkdir -p "$ESP_DIR/EFI/BOOT"

cargo run --quiet --manifest-path "$ROOT_DIR/apps/void-boot/Cargo.toml" --features internal-cli --bin void-boot-internal -- \
  validate-profiles "$PROFILE_FILE"
cargo run --quiet --manifest-path "$ROOT_DIR/apps/void-boot/Cargo.toml" --features internal-cli --bin void-boot-internal -- \
  render-menu "$PROFILE_FILE" "$MENU_DIR"

FONT_FILE="/usr/share/grub/unicode.pf2"
if [[ ! -s "$FONT_FILE" ]]; then
  FONT_FILE="$(find /usr/share/grub -name unicode.pf2 -type f -print -quit 2>/dev/null || true)"
fi
[[ -s "$FONT_FILE" ]] || { echo "GRUB unicode.pf2 font was not found." >&2; exit 1; }

# Include a serial marker so QEMU can verify that OVMF actually reached GRUB.
cat >> "$MENU_DIR/grub.cfg" <<'GRUB_TEST_MARKER'

insmod serial
serial --unit=0 --speed=115200
terminal_output gfxterm serial
echo "VOID_BOOT_MEDIA_READY"
GRUB_TEST_MARKER

grub-mkstandalone \
  --format=x86_64-efi \
  --output="$ESP_DIR/EFI/BOOT/BOOTX64.EFI" \
  --modules="part_gpt part_msdos fat ext2 normal configfile search search_fs_file linux loopback iso9660 gfxterm all_video font png serial terminal echo sleep halt reboot" \
  "boot/grub/grub.cfg=$MENU_DIR/grub.cfg" \
  "boot/grub/themes/void-boot/theme.txt=$MENU_DIR/themes/void-boot/theme.txt" \
  "boot/grub/fonts/unicode.pf2=$FONT_FILE"

# 1 GiB image: 128 MiB EFI System Partition + remaining FAT32 data partition.
IMAGE_SIZE_MIB="${VOID_BOOT_IMAGE_SIZE_MIB:-1024}"
if [[ ! "$IMAGE_SIZE_MIB" =~ ^[0-9]+$ ]] || (( IMAGE_SIZE_MIB < 512 )); then
  echo "VOID_BOOT_IMAGE_SIZE_MIB must be an integer of at least 512 MiB." >&2
  exit 1
fi
truncate -s "${IMAGE_SIZE_MIB}M" "$OUTPUT_IMAGE"
sgdisk --clear \
  --new=1:2048:+128M --typecode=1:ef00 --change-name=1:VOIDBOOT \
  --new=2:0:0 --typecode=2:0700 --change-name=2:VOIDDATA \
  "$OUTPUT_IMAGE" >/dev/null

ESP_OFFSET=$((2048 * 512))
DATA_START_SECTOR=$((2048 + 128 * 2048))
DATA_OFFSET=$((DATA_START_SECTOR * 512))
ESP_SPEC="$OUTPUT_IMAGE@@$ESP_OFFSET"
DATA_SPEC="$OUTPUT_IMAGE@@$DATA_OFFSET"

mformat -i "$ESP_SPEC" -F -v VOIDBOOT ::
mmd -i "$ESP_SPEC" ::/EFI
mmd -i "$ESP_SPEC" ::/EFI/BOOT
mcopy -i "$ESP_SPEC" "$ESP_DIR/EFI/BOOT/BOOTX64.EFI" ::/EFI/BOOT/BOOTX64.EFI

mformat -i "$DATA_SPEC" -F -v VOIDDATA ::

# Copy every unique ISO referenced by the profile catalog. In library mode,
# source files are selected by basename from the user-provided ISO folder.
declare -A CREATED_ISO_DIRS=()
for ISO_PATH in "${ISO_PATHS[@]}"; do
  ISO_PARENT="$(dirname "$ISO_PATH")"
  if [[ "$ISO_PARENT" != "." ]]; then
    IFS='/' read -r -a ISO_DIR_PARTS <<< "$ISO_PARENT"
    CURRENT_ISO_DIR=""
    for DIR_PART in "${ISO_DIR_PARTS[@]}"; do
      CURRENT_ISO_DIR="${CURRENT_ISO_DIR:+$CURRENT_ISO_DIR/}$DIR_PART"
      if [[ -z "${CREATED_ISO_DIRS[$CURRENT_ISO_DIR]:-}" ]]; then
        mmd -i "$DATA_SPEC" "::/$CURRENT_ISO_DIR"
        CREATED_ISO_DIRS[$CURRENT_ISO_DIR]=1
      fi
    done
  fi

  if (( ISO_LIBRARY_MODE == 1 )); then
    ISO_SOURCE="$SOURCE_ISO/$(basename "$ISO_PATH")"
  else
    ISO_SOURCE="$SOURCE_ISO"
  fi
  mcopy -i "$DATA_SPEC" "$ISO_SOURCE" "::/$ISO_PATH"
done

# Verify both partition signatures and the copied ISO without mounting anything.
sgdisk --verify "$OUTPUT_IMAGE" >/dev/null
mkdir -p "$WORK_DIR/verify"
for ISO_PATH in "${ISO_PATHS[@]}"; do
  if (( ISO_LIBRARY_MODE == 1 )); then
    ISO_SOURCE="$SOURCE_ISO/$(basename "$ISO_PATH")"
  else
    ISO_SOURCE="$SOURCE_ISO"
  fi
  VERIFY_FILE="$WORK_DIR/verify/$(basename "$ISO_PATH")"
  mcopy -i "$DATA_SPEC" "::/$ISO_PATH" "$VERIFY_FILE"
  cmp -s "$ISO_SOURCE" "$VERIFY_FILE" || {
    echo "ISO verification failed after copying: $ISO_PATH" >&2
    exit 1
  }
  echo "Verified ISO payload: $ISO_PATH"
done
test -s "$OUTPUT_IMAGE"
echo "Created UEFI Void Boot media image: $OUTPUT_IMAGE"
echo "Layout: GPT / 128 MiB FAT32 EFI System Partition / FAT32 ISO data partition."
if (( ISO_LIBRARY_MODE == 1 )); then
  echo "Imported ${#ISO_PATHS[@]} ISO file(s) from: $SOURCE_ISO"
fi
echo "This image is a regular file; no physical drive was modified."
