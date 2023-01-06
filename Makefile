RUSTFLAGS := --target x86_64-ruke.json
QEMUFLAGS := -m 128M -d int -serial stdio -vga std -no-reboot -no-shutdown

ASFLAGS := -felf64 -g

IMAGE := bin/image.iso

BUILDDIR := bin

ASMSRC := $(shell find src -name "*.s")
ASMOBJ := $(ASMSRC:.s=.o)

all: $(IMAGE)

%.o: %.s
	nasm $(ASFLAGS) $< -o $(addprefix $(BUILDDIR)/,$(notdir $@))

build: $(ASMOBJ)
	cargo build $(RUSTFLAGS)

$(IMAGE): build
	./build_kernel_image.sh

qemu: $(IMAGE)
	qemu-system-x86_64 -cdrom $(IMAGE) $(QEMUFLAGS) -boot d
