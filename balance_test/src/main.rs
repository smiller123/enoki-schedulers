extern crate libc;

use std::thread;
use std::time::{Duration, Instant};
use std::os::unix::thread::JoinHandleExt;

fn main() {
    let mut handles = Vec::new();
    for _j in 0..8 {
        let handle = thread::spawn(|| {
            let start = Instant::now();
            let mut val = 0;
            for _i in 0..u32::MAX/5 {
                val += 1;
            }
            let duration = start.elapsed();
            println!("time elapsed was {:?}", duration);
        });
        handles.push(handle);
    }
    while let Some(thread) = handles.pop() {
        //let pthread = thread.as_pthread_t();
        //unsafe {
        //    let mut set: libc::cpu_set_t = std::mem::zeroed();
        //    libc::CPU_SET(4, &mut set);
        //    libc::pthread_setaffinity_np(pthread, std::mem::size_of::<libc::cpu_set_t>(), &set);
        //}
        thread.join().unwrap();
    }
}
