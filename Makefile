RUSTFLAGS=--target x86_64-ruke.json
QEMUFLAGS := -m 128M -serial stdio -vga std -no-reboot -no-shutdown

IMAGE=bin/image.iso

all: $(IMAGE)

build:
	cargo build $(RUSTFLAGS)

$(IMAGE): build
	./build_kernel_image.sh

qemu: $(IMAGE)
	qemu-system-x86_64 -cdrom $(IMAGE) $(QEMUFLAGS) -boot d
