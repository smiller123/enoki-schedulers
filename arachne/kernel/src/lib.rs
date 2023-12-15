#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(alloc_layout_extra)]
#![feature(panic_info_message)]
#![feature(rustc_private)]
#![allow(improper_ctypes)]
#![feature(const_btree_new)]
#![feature(map_first_last)]
#![no_std]

extern crate alloc;
extern crate bento;
extern crate spin;
extern crate serde;

pub mod sched;

use bento::println;

use bento::kernel::raw;
use bento::scheduler_utils::*;

use bento::std::ffi::OsStr;
use bento::kernel::kobj::CStr;

use bento::spin_rs::RwLock;

use sched::BentoSched;

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::BTreeSet;

use core::str;

pub static mut BENTO_SCHED: BentoSched = BentoSched {
    q: None,
    qs: None,
    map: None,
    map2: None,
    user_q: None,
    rev_q: None,
    core_map: None,
    core_requests: None,
    proc_pids: None,
    pid_state: None,
    moved: None,
    need_timer: None,
    must_balance: None,
    evicting: None,
    assigned: None,
    cpu_running: None,
    clearing: None,
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
        BENTO_SCHED.user_q = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.rev_q = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.core_map = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.core_requests = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.proc_pids = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.pid_state = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.moved = Some(RwLock::new(BTreeSet::new()));
        BENTO_SCHED.need_timer = Some(RwLock::new(BTreeSet::new()));
        BENTO_SCHED.must_balance = Some(RwLock::new(BTreeSet::new()));
        BENTO_SCHED.evicting = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.assigned = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.cpu_running = Some(RwLock::new(BTreeSet::new()));
        BENTO_SCHED.clearing = Some(RwLock::new(false));
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
