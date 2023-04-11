//extern thread_priority;

use std::thread;
use thread_priority::*;
use std::time::{Duration, Instant};
use libc;

fn main() {
    let mut handles = Vec::new();
    for j in 0..5 {
        if j == 0 {
            let handle = thread::Builder::new()
                //.spawn(move || {
                .spawn_with_priority(ThreadPriority::Min, move |result| {
                println!("result {:?}", result);
                if j == 0 {
                    let res2 = set_current_thread_priority(ThreadPriority::Min);
                    println!("res2 {:?}", res2);
                };
                unsafe {
                let res2 = libc::setpriority(libc::PRIO_PROCESS, 0, libc::nice(20));
                println!("other res {}", res2); 
                }
                let start = Instant::now();
                let mut val = 0;
                for _i in 0..u32::MAX/10 {
                    val += 1;
                }
                let duration = start.elapsed();
                println!("time elapsed was {:?}", duration);
            }).unwrap();
            handles.push(handle);
        } else {
            let handle = thread::spawn(move || {
                //if j == 0 {
                //    set_current_thread_priority(ThreadPriority::Min);
                //}
                let start = Instant::now();
                let mut val = 0;
                for _i in 0..u32::MAX/10 {
                    val += 1;
                }
                let duration = start.elapsed();
                println!("time elapsed was {:?}", duration);
            });
            handles.push(handle);
        }
    }
    while let Some(thread) = handles.pop() {
        thread.join().unwrap();
    }
}
