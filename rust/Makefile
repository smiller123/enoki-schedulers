.DEFAULT_GOAL := kernel
.PHONY: kernel userspace clean
export KDIR ?= /lib/modules/$(shell uname -r)/build
CLANG ?= clang-11
ifeq ($(origin CC),default)
CC := ${CLANG}
endif

all:
	make kernel
	make userspace

kernel:
	$(MAKE) -C $(KDIR) M=$(CURDIR) CC=$(CC) CONFIG_CC_IS_CLANG=y

userspace:
	cp src/* userspace/src
	(cd userspace; cargo build --release --features "user")

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR) clean
	(cargo clean)
