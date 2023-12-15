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
use sched::CpuState;

use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;

use core::str;

pub static mut BENTO_SCHED: BentoSched = BentoSched {
    map: None,
    map2: None,
    state: None,
    state2: None,
    balancing: None,
    balancing_cpus: None,
    cpu_state: None,
    user_q: None,
    rev_q: None,
    locked: None,
};

#[no_mangle]
pub fn rust_main() {
    println!("Hello from Rust");
    unsafe {
        let mut cpu_state = BTreeMap::new();
        for i in 0..8 {
            let state = CpuState {
                weight: 0,
                inv_weight: 0,
                curr: None,
                set: BTreeSet::new(),
                free_time: 0,
                load: 0,
                capacity: 0xff,
            };
            cpu_state.insert(i, RwLock::new(state));
        }
        let state = CpuState {
            weight: 0,
            inv_weight: 0,
            curr: None,
            set: BTreeSet::new(),
            free_time: 0,
            load: 0,
            capacity: 0xff,
        };
        cpu_state.insert(u32::MAX, RwLock::new(state));
        BENTO_SCHED.map = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.map2 = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.state = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.state2 = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.cpu_state = Some(RwLock::new(cpu_state));
        BENTO_SCHED.user_q = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.rev_q = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.balancing = Some(RwLock::new(BTreeSet::new()));
        BENTO_SCHED.balancing_cpus = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.locked = Some(RwLock::new(BTreeSet::new()));
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

