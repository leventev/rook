#!/bin/bash

if [ "$EUID" -ne 0 ]
  then echo "please run $0 as root"
  exit
fi

TESTIMGPATH=bin/rook.img
SYSROOT=$(realpath root)

mkdir -p bin
dd if=/dev/zero of=$TESTIMGPATH bs=512 count=1024000
chmod 777 $TESTIMGPATH 
# ^^^^ this is a bad idea ^^^^
cat fdisk_inst.txt | fdisk -u $TESTIMGPATH
mkdir -p /mnt/ark_disk

losetup -o1048576 /dev/loop0 $TESTIMGPATH
mkfs.fat -F32 /dev/loop0
mount -t vfat /dev/loop0 /mnt/rook_disk

cp -RTf $SYSROOT /mnt/rook_disk
mkdir -p /mnt/rook_disk/boot/limine

#cp limine/BOOTX64.EFI /mnt/rook_disk/boot
cp conf/limine.cfg /mnt/rook_disk/boot/limine
cp limine/limine-bios.sys /mnt/rook_disk/boot/limine
cp target/x86_64-rook/debug/rook /mnt/rook_disk/boot

umount $TESTIMGPATH
losetup -d /dev/loop0

cd limine && make && cd ..
limine/limine bios-install $TESTIMGPATH
