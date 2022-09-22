//#![feature(lang_items)]
//#![feature(concat_idents)]
//#![feature(allocator_api)]
//#![feature(alloc_error_handler)]
//#![feature(alloc_layout_extra)]
//#![feature(panic_info_message)]
//#![feature(rustc_private)]
//#![allow(improper_ctypes)]
//#![feature(const_btree_new)]
//#![no_std]

//extern crate alloc;
//extern crate bento;
//extern crate spin;

#[macro_use]
extern crate nix;
extern crate memmap;

use std::env;
use std::fs::{File, OpenOptions};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::ffi::CStr;
use std::{thread, time};
use std::io::Write;

use memmap::MmapOptions;

//use bento::println;

//use bento::bindings as c;
//use bento::kernel::ffi;
//use bento::kernel::raw;
//use bento::scheduler_utils::*;
//
//use bento::std::ffi::OsStr;
//use bento::kernel::kobj::CStr;
//
//use spin::RwLock;
//
//use enclave::LocalEnclave;
//use ghost::Ghost;
//use sched::BentoSched;
//
//use alloc::collections::vec_deque::VecDeque;
//use alloc::collections::btree_map::BTreeMap;
//
//use core::mem;
//use core::str;
//use core::fmt::Debug;
//
//use ringbuffer::RingBuffer;

#[repr(C)]
pub struct bento_ioc_create_queue {
    elems: u32,
    flags: u32,
    mapsize: u64
}

#[repr(C)]
struct Queue {
    offset: u32,
    capacity: u32,
    head: u32,
    tail: u32,
}

impl Queue {
    fn dequeue(&mut self) -> Option<&str> {
        if self.head == self.tail {
            return None;
        }

        let index = self.tail & (self.capacity - 1);
        unsafe {
        let ptr = (self as *mut Queue as *mut i8).offset(self.offset as isize);
        let str_ptr = ptr.offset(index as isize * 256);
        let cstr = CStr::from_ptr(str_ptr);
        self.tail += 1;
        return Some(cstr.to_str().unwrap());
        }
    }
}

const BENTO_IOC_MAGIC: u8 = b'g';
const BENTO_IOC_TYPE_RECORD: u8 = 12;
ioctl_readwrite!(bento_create_record, BENTO_IOC_MAGIC, BENTO_IOC_TYPE_RECORD, bento_ioc_create_queue);

fn main() {
    //env_logger::init();
    let replay_name_arg = env::args_os().nth(1).unwrap();
    let replay_name = replay_name_arg.into_string().unwrap();
    println!("replay file name {}", replay_name);
    let mut f = File::create(replay_name).unwrap();

    let mut options = OpenOptions::new();
    let agent_file = options.read(true).write(true).open("/sys/fs/ghost/enclave_10/ctl").unwrap();
    let mut create_queue = bento_ioc_create_queue {
        elems: 32,
        flags: 0,
        mapsize: 0
    };
    let res = unsafe {
        bento_create_record(agent_file.as_raw_fd(), &mut create_queue as *mut bento_ioc_create_queue)
    };
    println!("res {:?}", res);
    let q_fd = res.unwrap();
    let q_file = unsafe {
        File::from_raw_fd(q_fd)
    };
    let mut mmap = unsafe {
        MmapOptions::new()
                     .len(create_queue.mapsize as usize)
                     .map_mut(&q_file)
                     .unwrap()
    };
    let q = unsafe {
        &mut *(mmap.as_mut_ptr() as *mut Queue)
    };
    let second = time::Duration::from_millis(1000);
    loop {
        let msg_ret = q.dequeue();
        if let Some(msg) = msg_ret {
            f.write(msg.as_bytes());
            f.write("\n".as_bytes());
        } else {
        //println!("got {:?}", msg);
        //if msg_ret.is_none() {
            thread::sleep(second);
        }
    }
    //let replay_arg_str = format!("fsname={}", disk_name.to_str().unwrap());
    //let replay_arg = fsname_arg_str.as_str();
}

//#[no_mangle]
//pub fn rust_main(record_file: *const i8) {
//        //println!("Hello from Rust");
////}
////impl bento::KernelModule for BentoGhostModule {
////    fn init() -> Result<Self, i32> {
//        println!("Hello from Rust");
//        println!("record_file {}", record_file as u64);
//        unsafe {
//            let name = unsafe { CStr::from_raw(record_file as *const raw::c_char) };
//            let name_str = OsStr::new(str::from_utf8(name.to_bytes_with_nul()).unwrap());
//            println!("file {:?}", name_str.to_str());
//        //println!("record_file {}", *record_file);
//        //println!("record_file {}", *record_file.offset(1));
//        BENTO_SCHED.q = Some(RwLock::new(VecDeque::new()));
//        BENTO_SCHED.map = Some(RwLock::new(BTreeMap::new()));
//        BENTO_SCHED.user_q = Some(RwLock::new(None));
//        BENTO_SCHED.register(record_file);
//        //let this_mod = BentoGhostModule {};
//        //Ok(this_mod)
//        }
//    //}
//}
//
//#[no_mangle]
//pub fn rust_exit() {
////impl Drop for BentoGhostModule {
//    //fn drop(&mut self) {
//        unsafe {
//        println!("Saying goodbye from Rust");
//        BENTO_SCHED.unregister();
//        println!("Goodbye from Rust");
//        }
//    //}
//}

//bento::kernel_module!(
//    BentoGhostModule,
//    author: b"Bento Contributors",
//    description: b"kernel module to replace the scheduler",
//    license: b"GPL"
//);
