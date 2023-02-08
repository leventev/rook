GCC_VERSION=12.2.0
BINUTILS_VERSION=2.40
GCC_URL=https://ftp.gnu.org/gnu/gcc/gcc-$(GCC_VERSION)/gcc-$(GCC_VERSION).tar.xz
BINUTILS_URL=https://ftp.gnu.org/gnu/binutils/binutils-$(BINUTILS_VERSION).tar.xz

.PHONY: build

all: extract build

download:
	wget $(GCC_URL)
	wget $(BINUTILS_URL)

extract:
	tar -xvf gcc-$(GCC_VERSION).tar.xz
	tar -xvf binutils-$(BINUTILS_VERSION).tar.xz
	patch -p0 < rook-gcc-$(GCC_VERSION).patch
	patch -p0 < rook-binutils-$(BINUTILS_VERSION).patch
	cd binutils-$(BINUTILS_VERSION) && autoreconf
	cd gcc-$(GCC_VERSION) && autoreconf

install:
	cd build && make install

clean:
	rm -rf build
