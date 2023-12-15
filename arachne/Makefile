.DEFAULT_GOAL := kernel
.PHONY: kernel clean
export KDIR ?= /lib/modules/$(shell uname -r)/build
CLANG ?= clang-11
ifeq ($(origin CC),default)
CC := ${CLANG}
endif

all:
	make kernel

kernel:
	cp src/* kernel/src
	touch kernel/.enoki_arachne.rust.o.cmd
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel CC=$(CC) CONFIG_CC_IS_CLANG=y

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR)/kernel clean
	(cd kernel; cargo clean)
