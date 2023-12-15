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
    let replay_name_arg = env::args_os().nth(1).unwrap();
    let replay_name = replay_name_arg.into_string().unwrap();
    println!("replay file name {}", replay_name);
    let mut f = File::create(replay_name).unwrap();

    let mut options = OpenOptions::new();
    let agent_file = options.read(true).write(true).open("/sys/fs/ghost/ctl").unwrap();
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
            thread::sleep(second);
        }
    }
}
