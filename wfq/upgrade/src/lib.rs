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

use bento::scheduler_utils::*;

use sched::BentoSched;

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
pub fn rust_main(record_file: *const i8) {
    println!("Hello from Rust");
    println!("record_file {}", record_file as u64);
    unsafe {
        BENTO_SCHED.reregister();
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
