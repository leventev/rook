git clone https://github.com/limine-bootloader/limine.git --branch=v4.x-branch-binary --depth=1

make -C limine

mkdir -p iso_root
mkdir -p bin

cp -v target/x86_64-ruke/debug/ruke conf/limine.cfg limine/limine.sys \
    limine/limine-cd.bin limine/limine-cd-efi.bin iso_root

xorriso -as mkisofs -b limine-cd.bin \
    -no-emul-boot -boot-load-size 4 -boot-info-table \
    --efi-boot limine-cd-efi.bin \
    -efi-boot-part --efi-boot-image --protective-msdos-label \
    iso_root -o bin/image.iso

./limine/limine-deploy bin/image.iso