use std::thread;
use std::time::{Duration, Instant};

fn main() {
    let mut handles = Vec::new();
    for _j in 0..5 {
        let handle = thread::spawn(|| {
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
    while let Some(thread) = handles.pop() {
        thread.join().unwrap();
    }
}
