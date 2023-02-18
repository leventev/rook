ARCH=x86_64

CARGOFLAGS=--target x86_64-rook.json
RUSTFLAGS=-Cforce-frame-pointers=yes
QEMUFLAGS=-m 128M -serial stdio -vga std -no-reboot -no-shutdown\
-drive file=$(IMAGE),if=ide,media=disk,format=raw\

SYSROOT=$(shell realpath root)

CROSSDIR=$(HOME)/opt/cross

BINUTILS_COMPFLAGS=--target=x86_64-rook --prefix=$(CROSSDIR)\
	--with-sysroot=$(SYSROOT) --disable-nls --disable-werror

GCC_COMPFLAGS=--target=x86_64-rook --prefix=$(CROSSDIR)\
	--with-sysroot=$(SYSROOT) --disable-nls --disable-werror --enable-languages=c

.PHONY: libc

BUILDDIR=bin
IMAGE=$(BUILDDIR)/rook.img

ASMSRC=$(shell find src -name "*.s")
ASMOBJ=$(ASMSRC:.s=.o)

BINUTILS_VER=2.40
GCC_VER=12.2.0

all: image

$(BUILDDIR):
	mkdir -p $(BUILDDIR)

build: $(BUILDDIR)
	cargo build $(CARGOFLAGS)

image: build
	sudo ./make_disk.sh

qemu: image
	qemu-system-x86_64 $(QEMUFLAGS)

qemu-debug: QEMUFLAGS += -s -S
qemu-debug: qemu

sysroot:
	mkdir -p $(SYSROOT)/usr/include
	mkdir -p $(SYSROOT)/usr/lib

copy-libc-headers: sysroot
	cp -rfT libc/include/public $(SYSROOT)/usr/include

libc: copy-libc-headers
#	cp include/ark/arch/x86/syscalls.h libc/include/public
# TODO: remove ^
	cd libc && $(MAKE)
	cp libc/bin/libc.a $(SYSROOT)/usr/lib/
	cp libc/bin/arch/$(ARCH)/crt0.s.o $(SYSROOT)/usr/lib/crt0.o

build-binutils:
	mkdir -p binutils_build
	cd binutils_build\
	&& ../toolchain/binutils-$(BINUTILS_VER)/configure $(BINUTILS_COMPFLAGS)\
	&& $(MAKE) && make install
	rm -rf binutils_build

build-gcc:
	mkdir -p gcc_build
	cd gcc_build\
	&& ../toolchain/gcc-$(GCC_VER)/configure $(GCC_COMPFLAGS)\
	&& $(MAKE) all-gcc all-target-libgcc\
	&& make install-gcc install-target-libgcc
	rm -rf gcc_build

clean:
	rm -rf $(BUILDDIR)
