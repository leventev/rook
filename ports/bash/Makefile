BASH_VERSION=5.2
BASH_URL=https://ftp.gnu.org/gnu/bash/bash-$(BASH_VERSION).tar.gz
CONFFLAGS=--host=x86_64-rook --with-bash-malloc=false --disable-nls --enable-history=false

.PHONY: build

all: extract build

download:
	wget $(BASH_URL)

extract:
	tar -xvf bash-$(BASH_VERSION).tar.gz
	patch -p0 < bash-$(BASH_VERSION).patch
	cd bash-$(BASH_VERSION) && autoconf

build:
	mkdir -p build
	cd build && ../bash-$(BASH_VERSION)/configure $(CONFFLAGS)
	sed -i "s/#define GETCWD_BROKEN 1/#undef GETCWD_BROKEN/g" build/config.h
	cd build && make -j10

install:
	cp build/bash ../../root/bin/bash

clean:
	rm -rf build