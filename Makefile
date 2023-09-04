ARCH=x86_64

CARGOFLAGS=
RUSTFLAGS=-Cforce-frame-pointers=yes
QEMUFLAGS=-m 128M -serial stdio -vga std -no-reboot -no-shutdown\
-drive file=$(IMAGE),if=ide,media=disk,format=raw\

SYSROOT=$(shell realpath root)

CROSSDIR=$(HOME)/opt/cross

BINUTILS_COMPFLAGS=--target=x86_64-rook --prefix=$(CROSSDIR)\
	--with-sysroot=$(SYSROOT) --disable-nls --disable-werror

GCC_COMPFLAGS=--target=x86_64-rook --prefix=$(CROSSDIR)\
	--with-sysroot=$(SYSROOT) --disable-nls --disable-werror --enable-languages=c,c++

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

$(SYSROOT):
	mkdir -p $(SYSROOT)/usr/include
	mkdir -p $(SYSROOT)/usr/lib
	mkdir -p $(SYSROOT)/bin

copy-libc-headers: $(SYSROOT)
	cd mlibc && meson setup --cross x86_64-rook.txt -Ddefault_library=static -Dheaders_only=true -Dprefix=$(SYSROOT)/usr build-rook
	cd mlibc/build-rook && meson install
	rm -rf mlibc/build_rook

libc: $(SYSROOT)
	cd mlibc && meson setup --cross x86_64-rook.txt -Ddefault_library=static -Dprefix=$(SYSROOT)/usr build-rook
	cd mlibc/build-rook && meson compile && meson install

build-binutils: copy-libc-headers
	mkdir -p binutils_build
	cd binutils_build\
	&& ../toolchain/binutils-$(BINUTILS_VER)/configure $(BINUTILS_COMPFLAGS)\
	&& $(MAKE) && make install
	rm -rf binutils_build

build-gcc: copy-libc-headers
	mkdir -p gcc_build
	cd gcc_build\
	&& ../toolchain/gcc-$(GCC_VER)/configure $(GCC_COMPFLAGS)\
	&& $(MAKE) all-gcc all-target-libgcc\
	&& make install-gcc install-target-libgcc
	rm -rf gcc_build

clean:
	rm -rf $(BUILDDIR)
