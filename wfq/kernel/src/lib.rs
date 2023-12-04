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
#![feature(let_chains)]
#![no_std]

extern crate alloc;
extern crate bento;
extern crate spin;
extern crate serde;

//pub mod enclave;
//pub mod ghost;
pub mod sched;

use bento::println;

use bento::bindings as c;
use bento::kernel::ffi;
use bento::kernel::raw;
use bento::scheduler_utils::*;
use bento::kernel::cpu::*;
//use rbtree::RBTree;

use bento::std::ffi::OsStr;
use bento::kernel::kobj::CStr;

//use spin::RwLock;
use bento::spin_rs::RwLock;

use sched::BentoSched;
use sched::CpuState;

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;
//use slice_rbtree::tree::{tree_size, RBTree, TreeParams};

use core::mem;
use core::str;
use core::fmt::Debug;

use ringbuffer::RingBuffer;

pub static mut BENTO_SCHED: BentoSched = BentoSched {
//    sets_list: None,
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
        //println!("Hello from Rust");
//}
//impl bento::KernelModule for BentoGhostModule {
//    fn init() -> Result<Self, i32> {
        println!("Hello from Rust");
        println!("record_file {}", record_file as u64);
        unsafe {
            let name = unsafe { CStr::from_raw(record_file as *const raw::c_char) };
            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
            println!("file {:?}", name_str.to_str());
        //println!("record_file {}", *record_file);
        //println!("record_file {}", *record_file.offset(1));
        let mut cpu_state = BTreeMap::new();
        for i in 0..num_online_cpus() as u32 {
            let state = CpuState {
                weight: 0,
                inv_weight: 0,
                curr: None,
              //  leftmost: (u64::MAX, u64::MAX),
                set: BTreeSet::new(),
                free_time: 0,
                load: 0,
                capacity: 0xff,
                should_report: 0,
                //set: RBTree::new(),
             //   fake_set: BTreeSet::new(),
            };
            cpu_state.insert(i, RwLock::new(state));
        }
        let state = CpuState {
            weight: 0,
            inv_weight: 0,
            curr: None,
            //leftmost: (u64::MAX, u64::MAX),
            set: BTreeSet::new(),
            free_time: 0,
            load: 0,
            capacity: 0xff,
            should_report: 0,
            //set: RBTree::new(),
            //fake_set: BTreeSet::new(),
        };
        cpu_state.insert(u32::MAX, RwLock::new(state));
//        BENTO_SCHED.sets_list = Some(RwLock::new(sets_list));
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
        //let this_mod = BentoGhostModule {};
        //Ok(this_mod)
        }
    //}
}

#[no_mangle]
pub fn rust_exit() {
//impl Drop for BentoGhostModule {
    //fn drop(&mut self) {
        unsafe {
        println!("Saying goodbye from Rust");
        BENTO_SCHED.unregister();
        println!("Goodbye from Rust");
        }
    //}
}

//bento::kernel_module!(
//    BentoGhostModule,
//    author: b"Bento Contributors",
//    description: b"kernel module to replace the scheduler",
//    license: b"GPL"
//);
