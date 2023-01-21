CARGOFLAGS=--target x86_64-rook.json
RUSTFLAGS=-Cforce-frame-pointers=yes
ASFLAGS=-felf64 -g
QEMUFLAGS=-m 128M -d int -serial stdio -vga std -no-reboot -no-shutdown\
-drive file=$(IMAGE),if=ide,media=disk,format=raw\

BUILDDIR=bin
IMAGE=$(BUILDDIR)/rook.img

ASMSRC=$(shell find src -name "*.s")
ASMOBJ=$(ASMSRC:.s=.o)

all: image

$(BUILDDIR):
	mkdir -p $(BUILDDIR)

%.o: %.s
	nasm $(ASFLAGS) $< -o $(addprefix $(BUILDDIR)/,$(notdir $@))

build: $(BUILDDIR) $(ASMOBJ)
	cargo build $(CARGOFLAGS)

image: build
	sudo ./make_disk.sh

qemu: image
	qemu-system-x86_64 $(QEMUFLAGS)

qemu-debug: QEMUFLAGS += -s -S
qemu-debug: qemu

clean:
	rm -rf $(BUILDDIR)
