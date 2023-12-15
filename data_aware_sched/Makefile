.DEFAULT_GOAL := kernel
.PHONY: kernel kernel-record record replay clean
export KDIR ?= /lib/modules/$(shell uname -r)/build
CLANG ?= clang-11
ifeq ($(origin CC),default)
CC := ${CLANG}
endif

all:
	make kernel
	make userspace

kernel:
	cp src/* kernel/src
	touch kernel/.enoki_data.rust.o.cmd
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel CC=$(CC) CONFIG_CC_IS_CLANG=y

kernel-record:
	cp src/* kernel-record/src
	touch kernel-record/.enoki_data.rust.o.cmd
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel-record CC=$(CC) CONFIG_CC_IS_CLANG=y

record:
	(cd record; cargo build --release)

replay:
	cp src/* replay/src
	(cd replay; cargo build --release --features "replay")

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel clean
	(cd kernel; cargo clean)
	(cd kernel-record; cargo clean)
	(cd record; cargo clean)
	(cd replay; cargo clean)
