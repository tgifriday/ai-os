#!/bin/bash
# QEMU VM image build script for AIOS
# This creates a minimal bootable Linux image with AIOS as the userspace layer.
#
# Prerequisites:
#   - qemu-system-x86_64 installed
#   - debootstrap installed (for Debian-based host)
#   - Rust toolchain with x86_64-unknown-linux-gnu target
#
# Usage: ./scripts/build-qemu-image.sh
#
# The resulting image can be booted with:
#   qemu-system-x86_64 -hda aios.img -m 2G -enable-kvm -nographic

set -euo pipefail

IMAGE_SIZE="2G"
IMAGE_FILE="aios.img"
MOUNT_POINT="/tmp/aios-mount"

echo "=== AIOS QEMU Image Builder ==="
echo "This is a blueprint for building a bootable AIOS VM image."
echo ""
echo "Steps to build manually:"
echo ""
echo "1. Cross-compile AIOS for x86_64 Linux:"
echo "   rustup target add x86_64-unknown-linux-gnu"
echo "   cargo build --workspace --release --target x86_64-unknown-linux-gnu"
echo ""
echo "2. Create a disk image:"
echo "   qemu-img create -f raw $IMAGE_FILE $IMAGE_SIZE"
echo "   mkfs.ext4 $IMAGE_FILE"
echo ""
echo "3. Mount and populate with minimal Linux + AIOS:"
echo "   mkdir -p $MOUNT_POINT"
echo "   sudo mount -o loop $IMAGE_FILE $MOUNT_POINT"
echo "   sudo debootstrap --variant=minbase bookworm $MOUNT_POINT"
echo "   sudo cp target/x86_64-unknown-linux-gnu/release/aios-shell $MOUNT_POINT/bin/"
echo "   sudo cp target/x86_64-unknown-linux-gnu/release/aios-init $MOUNT_POINT/sbin/init"
echo "   sudo cp -r config/ $MOUNT_POINT/etc/aios/"
echo "   sudo umount $MOUNT_POINT"
echo ""
echo "4. Install bootloader (GRUB) and kernel into the image"
echo ""
echo "5. Boot with QEMU:"
echo "   qemu-system-x86_64 -hda $IMAGE_FILE -m 2G -enable-kvm -nographic"
echo ""
echo "For development, use Docker instead:"
echo "   docker build -t aios ."
echo "   docker run -it aios"
