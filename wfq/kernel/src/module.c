/*
 * SPDX-License-Identifier: GPL-2.0 OR MIT
 *
 * Copyright (c) 2020 Samantha Miller, Kaiyuan Zhang, Danyang Zhuo, Tom
      Anderson, Ang Chen, University of Washington
 * From tsgates/rust.ko, but I don't know how to convey that
 */

#include <linux/init.h>
#include <linux/module.h>
#include <linux/kernel.h>
#include <linux/slab.h>
#include <linux/bug.h>

char __morestack[1024];
char _GLOBAL_OFFSET_TABLE_;

static char record_file[100] = "";
module_param_string(record_file, record_file, 100, 0400);
MODULE_PARM_DESC(record_file, "File name of record-replay file");

void abort(void)
{
    BUG();
}

extern void rust_main(char *);
extern void rust_exit(void);

static int xv6fs_init(void)
{
    rust_main(record_file);
    return 0;
}

static void xv6fs_exit(void)
{
    rust_exit();
}

module_init(xv6fs_init);
module_exit(xv6fs_exit);

MODULE_LICENSE("GPL");
