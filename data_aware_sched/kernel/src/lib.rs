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

//pub mod enclave;
//pub mod ghost;
pub mod sched;

use bento::println;

use bento::bindings as c;
use bento::kernel::ffi;
use bento::kernel::raw;
use bento::scheduler_utils::*;

use bento::std::ffi::OsStr;
use bento::kernel::kobj::CStr;

//use spin::RwLock;
use bento::spin_rs::RwLock;

use sched::BentoSched;

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;

use core::mem;
use core::str;
use core::fmt::Debug;

use ringbuffer::RingBuffer;

pub static mut BENTO_SCHED: BentoSched = BentoSched {
    q: None,
    map: None,
    user_q: None,
    rev_q: None,
    hint_to_core: None,
    pid_to_hint: None,
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
        BENTO_SCHED.q = Some(RwLock::new(VecDeque::new()));
        BENTO_SCHED.map = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.user_q = Some(RwLock::new(None));
        BENTO_SCHED.rev_q = Some(RwLock::new(None));
        BENTO_SCHED.hint_to_core = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.pid_to_hint = Some(RwLock::new(BTreeMap::new()));
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
