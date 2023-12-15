#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(alloc_layout_extra)]
#![feature(panic_info_message)]
#![feature(rustc_private)]
#![allow(improper_ctypes)]
#![feature(const_btree_new)]
#![no_std]

extern crate alloc;
extern crate bento;
extern crate spin;
extern crate serde;

pub mod sched;

use bento::println;

use bento::bindings as c;
use bento::kernel::ffi;
use bento::kernel::raw;
use bento::scheduler_utils::*;

use bento::std::ffi::OsStr;
use bento::kernel::kobj::CStr;

use bento::spin_rs::RwLock;

use sched::BentoSched;

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;

use core::mem;
use core::str;
use core::fmt::Debug;

use ringbuffer::RingBuffer;

pub static mut BENTO_SCHED: BentoSched = BentoSched {
    qs: None,
    q: None,
    map: None,
    map2: None,
    moved: None,
    pid_state: None,
    cpu_running: None,
    user_q: None,
    rev_q: None,
    timing: None,
};

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    unsafe {
        let mut qs = BTreeMap::new();
        for i in 0..8 {
            qs.insert(i, RwLock::new(VecDeque::new()));
        }
        qs.insert(u32::MAX, RwLock::new(VecDeque::new()));
        BENTO_SCHED.qs = Some(RwLock::new(qs));
        BENTO_SCHED.q = Some(RwLock::new(VecDeque::new()));
        BENTO_SCHED.map = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.map2 = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.moved = Some(RwLock::new(BTreeSet::new()));
        BENTO_SCHED.pid_state = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.cpu_running = Some(RwLock::new(BTreeSet::new()));
        BENTO_SCHED.user_q = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.rev_q = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.timing = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.register();
    }
}

#[no_mangle]
pub fn rust_exit() {
    unsafe {
        println!("Saying goodbye from Rust");
        BENTO_SCHED.unregister();
        println!("Goodbye from Rust");
    }
}

