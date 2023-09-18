.DEFAULT_GOAL := kernel
.PHONY: kernel upgrade kernel-record record replay clean
export KDIR ?= /lib/modules/$(shell uname -r)/build
CLANG ?= clang-11
ifeq ($(origin CC),default)
CC := ${CLANG}
endif

all:
	make kernel
	make upgrade
	make kernel-record
	make record
	make replay

kernel:
	cp src/* kernel/src
	touch kernel/.enoki_wfq.rust.o.cmd
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel CC=$(CC) CONFIG_CC_IS_CLANG=y

upgrade:
	cp src/* upgrade/src
	touch upgrade/.bento_ghost2.rust.o.cmd
	$(MAKE) -C $(KDIR) M=$(CURDIR)/upgrade CC=$(CC) CONFIG_CC_IS_CLANG=y

kernel-record:
	cp src/* kernel-record/src
	touch kernel-record/.bento_ghost.rust.o.cmd
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel-record CC=$(CC) CONFIG_CC_IS_CLANG=y

record:
	(cd record; cargo build --release)

replay:
	cp src/* replay/src
	(cd replay; cargo build --release --features "replay")

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel clean
	(cd kernel; cargo clean)
	$(MAKE) -C $(KDIR) M=$(CURDIR)/upgrade clean
	(cd upgrade; cargo clean)
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel-record clean
	(cd kernel-record; cargo clean)
	(cd record; cargo clean)
	(cd replay; cargo clean)
