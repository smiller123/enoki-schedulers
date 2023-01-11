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

#![feature(let_chains)]

extern crate alloc;
extern crate scheduler_utils;
//extern crate spin;
extern crate core;
extern crate serde;

pub mod enclave;
pub mod ghost;
pub mod sched;

use std::env;
//use spin::RwLock;

use sched::BentoSched;
use scheduler_utils::BentoScheduler;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use scheduler_utils::spin_rs::RwLock;

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;
use std::collections::HashMap;
use std::thread;
use std::sync::Arc;

use scheduler_utils::ringbuffer::RingBuffer;
use scheduler_utils::Schedulable;

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn main() {
    let replay_name_arg = env::args_os().nth(1).unwrap();
    let replay_name = replay_name_arg.into_string().unwrap();
    let mut replay_name2 = Some(replay_name.clone());
    println!("replay name {}", replay_name);

    let mut lock_lines = VecDeque::new();
    let mut found_locks = VecDeque::new();
    let mut found_lock = true;
    while found_lock {
        let mut lock_addr: Option<u64> = None;
        let mut this_lock_lines = VecDeque::new();
        let lines = read_lines(replay_name.clone()).unwrap();
        found_lock = false;
        for line_res in lines {
            if let Ok(line) = line_res {
                //println!("line is {}", line);
                let line_split = line.clone();
                let mut split = line_split.split_whitespace();
                let command = split.next().unwrap();
                if lock_addr.is_none() && command == "lock_new:" {
                    let this_lock = Some(split.nth(1).unwrap().parse().unwrap());
                    if !found_locks.contains(&this_lock) {
                        lock_addr = this_lock;
                        found_locks.push_back(lock_addr);
                        found_lock = true;
                    }
                }
                if command.contains("_lock") && lock_addr.is_some() {
                    if split.nth(1).unwrap().parse::<u64>().unwrap() == lock_addr.unwrap() {
                        this_lock_lines.push_back(line);
                    }
                }
            }
        }
        lock_lines.push_back(this_lock_lines);
    }
    let lines = read_lines(replay_name).unwrap();


    let bento_sched = Arc::new(BentoSched {
        q: Some(RwLock::new(VecDeque::new(), lock_lines.pop_front().unwrap())),
        map: Some(RwLock::new(BTreeMap::new(), lock_lines.pop_front().unwrap())),
        user_q: Some(RwLock::new(None, lock_lines.pop_front().unwrap())),
        rev_q: Some(RwLock::new(None, lock_lines.pop_front().unwrap())),
    });
    let mut pnt_response_hands = HashMap::new();
    let mut select_response_hands = HashMap::new();
    let mut balance_response_hands = HashMap::new();
    for line_res in lines {
        if let Ok(line) = line_res {
            let mut split = line.split_whitespace();
            // First argument will be at pos 3 after the next(),
            // then each argument is every 2 after that
            match split.next() {
                Some("pnt:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let cpu_str = split.nth(3).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.pick_next_task(cpu)
                    }).unwrap();
                    pnt_response_hands.insert(cpu, handle);
                },
                Some("pnt_ret:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let cpu_str = split.next().unwrap();
                    let cpu: i32 = cpu_str.parse().unwrap();
                    if pnt_response_hands.get(&cpu).is_none() {
                        println!("Found pick next task response with no corresponding call");
                    }
                    let handle = pnt_response_hands.remove(&cpu).unwrap();
                    let response = handle.join().unwrap();
                    let got = alloc::format!("{:?}", response);
                    let expected = split.next().unwrap();
                    if got != expected {
                        println!("Expected pick next task response {}, got {}",
                                 expected, got);
                    }
                    if split.find(|&x| x == "Error").is_some() && let Some(sched) = response  {
                        bento_sched.pnt_err(sched);
                    }
                },
                Some("dead:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.task_dead(pid)
                    }).unwrap();
                    //bento_sched.task_dead(pid);
                },
                Some("blocked:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let runtime_str = split.nth(1).unwrap();
                    let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                    let cpu_seq_str = split.nth(1).unwrap();
                    let cpu_seqnum: u64 = cpu_seq_str[0..cpu_seq_str.len()-1].parse().unwrap();
                    let cpu_str = split.nth(1).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                    let from_str = split.nth(1).unwrap();
                    let from_switchto: i8 = from_str[0..from_str.len()].parse().unwrap();
                    let sched = Schedulable {
                        pid: pid,
                        cpu: cpu as u32,
                    };
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.task_blocked(pid, runtime, cpu_seqnum, cpu, from_switchto, sched)
                    }).unwrap();
                    //bento_sched.task_blocked(pid, runtime, cpu_seqnum, cpu, from_switchto, sched);
                },
                Some("wakeup:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let agent_str = split.nth(1).unwrap();
                    let agent_data: u64 = agent_str[0..agent_str.len()-1].parse().unwrap();
                    let defer_str = split.nth(1).unwrap();
                    let deferrable: u8 = defer_str[0..defer_str.len()-1].parse().unwrap();
                    let last_str = split.nth(1).unwrap();
                    let last_ran_cpu: i32 = last_str[0..last_str.len()-1].parse().unwrap();
                    let wake_up_str = split.nth(1).unwrap();
                    let wake_up_cpu: i32 = wake_up_str[0..wake_up_str.len()-1].parse().unwrap();
                    let waker_str = split.nth(1).unwrap();
                    let waker_cpu: i32 = waker_str[0..waker_str.len()].parse().unwrap();
                    let sched = Schedulable {
                        pid: pid,
                        cpu: wake_up_cpu as u32,
                    };
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.task_wakeup(pid, agent_data, deferrable > 0,
                                            last_ran_cpu, wake_up_cpu, waker_cpu, sched);
                    }).unwrap();
                    //bento_sched.task_wakeup(pid, agent_data, deferrable > 0,
                     //                       last_ran_cpu, wake_up_cpu, waker_cpu, sched);
                },
                Some("new:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let runtime_str = split.nth(1).unwrap();
                    println!("{}", runtime_str);
                    let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                    let runnable_str = split.nth(1).unwrap();
                    let runnable: u16 = runnable_str[0..runnable_str.len()-1].parse().unwrap();
                    let sched = Schedulable {
                        pid: pid,
                        cpu: u32::MAX,
                    };
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.task_new(pid, runtime, runnable, sched);
                    }).unwrap();
                    //bento_sched.task_new(pid, runtime, runnable, sched);
                },
                Some("preempt:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let runtime_str = split.nth(1).unwrap();
                    let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                    let cpu_seq_str = split.nth(1).unwrap();
                    let cpu_seqnum: u64 = cpu_seq_str[0..cpu_seq_str.len()-1].parse().unwrap();
                    let cpu_str = split.nth(1).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                    let from_str = split.nth(1).unwrap();
                    let from_switchto: i8 = from_str[0..from_str.len()-1].parse().unwrap();
                    let latched_str = split.nth(1).unwrap();
                    let was_latched: i8 = latched_str[0..latched_str.len()].parse().unwrap();
                    let sched = Schedulable {
                        pid: pid,
                        cpu: cpu as u32,
                    };
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.task_preempt(pid, runtime, cpu_seqnum,
                                             cpu, from_switchto, was_latched, sched);
                    }).unwrap();
                    //bento_sched.task_preempt(pid, runtime, cpu_seqnum,
                     //                        cpu, from_switchto, was_latched, sched);
                },
                Some("yield:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let runtime_str = split.nth(1).unwrap();
                    let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                    let cpu_seq_str = split.nth(1).unwrap();
                    let cpu_seqnum: u64 = cpu_seq_str[0..cpu_seq_str.len()-1].parse().unwrap();
                    let cpu_str = split.nth(1).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                    let from_str = split.nth(1).unwrap();
                    let from_switchto: i8 = from_str[0..from_str.len()].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.task_yield(pid, runtime, cpu_seqnum,
                                             cpu, from_switchto);
                    }).unwrap();
                    //bento_sched.task_yield(pid, runtime, cpu_seqnum,
                     //                        cpu, from_switchto);
                },
                Some("departed:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let cpu_seq_str = split.nth(1).unwrap();
                    let cpu_seqnum: u64 = cpu_seq_str[0..cpu_seq_str.len()-1].parse().unwrap();
                    let cpu_str = split.nth(1).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                    let from_str = split.nth(1).unwrap();
                    let from_switchto: i8 = from_str[0..from_str.len()-1].parse().unwrap();
                    let curr_str = split.nth(1).unwrap();
                    let was_current: i8 = from_str[0..from_str.len()].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.task_departed(pid, cpu_seqnum,
                                             cpu, from_switchto, was_current);
                    }).unwrap();
                    //bento_sched.task_departed(pid, cpu_seqnum,
                     //                        cpu, from_switchto, was_current);
                },
                Some("switchto:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let runtime_str = split.nth(1).unwrap();
                    let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                    let cpu_seq_str = split.nth(1).unwrap();
                    let cpu_seqnum: u64 = cpu_seq_str[0..cpu_seq_str.len()-1].parse().unwrap();
                    let cpu_str = split.nth(1).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.task_switchto(pid, runtime, cpu_seqnum, cpu);
                    }).unwrap();
                    //bento_sched.task_switchto(pid, runtime, cpu_seqnum, cpu);
                },
                Some("affinity:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.task_affinity_changed(pid);
                    }).unwrap();
                    //bento_sched.task_affinity_changed(pid);
                },
                Some("latched:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let commit_str = split.nth(1).unwrap();
                    let commit_time: u64 = commit_str[0..commit_str.len()-1].parse().unwrap();
                    let cpu_seq_str = split.nth(1).unwrap();
                    let cpu_seqnum: u64 = cpu_seq_str[0..cpu_seq_str.len()-1].parse().unwrap();
                    let cpu_str = split.nth(1).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                    let latched_str = split.nth(1).unwrap();
                    let latched_preempt: i8 = latched_str[0..latched_str.len()].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.task_latched(pid, commit_time, cpu_seqnum, cpu, latched_preempt);
                    }).unwrap();
                    //bento_sched.task_latched(pid, commit_time, cpu_seqnum, cpu, latched_preempt);
                },
                Some("tick:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let cpu_str = split.nth(3).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.cpu_tick(cpu);
                    }).unwrap();
                    //bento_sched.cpu_tick(cpu);
                },
                Some("not_idle:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let cpu_str = split.nth(3).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                    let pid_str = split.nth(1).unwrap();
                    let next_pid: u64 = pid_str[0..pid_str.len()].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.cpu_not_idle(cpu, next_pid);
                    }).unwrap();
                    //bento_sched.cpu_not_idle(cpu, next_pid);
                },
                Some("select_rq:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.select_task_rq(pid)
                    }).unwrap();
                    select_response_hands.insert(pid, handle);
                    //let cpu = bento_sched.select_task_rq(pid);
                    //select_response.insert(pid, cpu);
                },
                Some("select_rq_ret:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.next().unwrap();
                    let pid: u64 = pid_str.parse().unwrap();
                    if select_response_hands.get(&pid).is_none() {
                        println!("Found select task rq response with no corresponding call");
                    }
                    let handle = select_response_hands.remove(&pid).unwrap();
                    let response = handle.join().unwrap();
                    //let response = select_response.get(&pid).unwrap();
                    let got = alloc::format!("{}", response);
                    let expected = split.next().unwrap();
                    if got != expected {
                        println!("Expected select task rq response {}, got {}",
                                 expected, got);
                    }
                    let sched = Schedulable {
                        pid: pid,
                        cpu: response as u32,
                    };
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.selected_task_rq(sched);
                    }).unwrap();
                    //bento_sched.selected_task_rq(sched);
                },
                Some("migrate_rq:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let next_str = split.nth(1).unwrap();
                    let next_cpu: i32 = next_str[0..next_str.len()].parse().unwrap();
                    let sched = Schedulable {
                        pid: pid,
                        cpu: next_cpu as u32,
                    };
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.migrate_task_rq(pid, sched);
                    }).unwrap();
                    //bento_sched.migrate_task_rq(pid, sched);
                },
                Some("balance:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let cpu_str = split.nth(3).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.balance(cpu)
                    }).unwrap();
                    //let res = bento_sched.balance(cpu);
                    balance_response_hands.insert(cpu, handle);
                    //balance_response.insert(cpu, res);
                },
                Some("balance_ret:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let cpu_str = split.next().unwrap();
                    let cpu: i32 = cpu_str.parse().unwrap();
                    if balance_response_hands.get(&cpu).is_none() {
                        println!("Found balance response with no corresponding call");
                    }
                    let handle = balance_response_hands.remove(&cpu).unwrap();
                    let response = handle.join().unwrap();
                    //let response = balance_response.get(&cpu).unwrap();
                    let got = alloc::format!("{:?}", response);
                    let expected = split.next().unwrap();
                    if got != expected {
                        println!("Expected balance response {}, got {}",
                                 expected, got);
                    }
                    //balance_response.remove(&cpu);
                },
                Some("create_queue") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    if replay_name2.is_some() {
                        println!("registering queue");
                        let fname = replay_name2.take().unwrap();
                        let buffer = RingBuffer::from_file(fname);
                        let bento_clone = Arc::clone(&bento_sched);
                        let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                            bento_clone.register_queue(buffer);
                        }).unwrap();
                        //bento_sched.register_queue(buffer);
                    }
                },
                Some("create_reverse_queue") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    if replay_name2.is_some() {
                        println!("registering reverse queue");
                        let fname = replay_name2.take().unwrap();
                        let buffer = RingBuffer::from_file(fname);
                        let bento_clone = Arc::clone(&bento_sched);
                        let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                            bento_clone.register_reverse_queue(buffer);
                        }).unwrap();
                        //bento_sched.register_reverse_queue(buffer);
                    }
                },
                Some("enter_queue:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let entries_str = split.nth(3).unwrap();
                    let entries: u32 = entries_str[0..entries_str.len()].parse().unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.enter_queue(entries);
                    }).unwrap();
                    //bento_sched.enter_queue(entries);
                },
                Some("unregister_queue:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.unregister_queue();
                    }).unwrap();
                    //bento_sched.unregister_queue();
                },
                Some("unregister_reverse_queue:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.unregister_rev_queue();
                    }).unwrap();
                    //bento_sched.unregister_rev_queue();
                },
                Some("dequeue2:") => {
                    let kpid_str = split.next().unwrap();
                    let kpid: u32 = kpid_str.parse().unwrap();

                    let mut buf = [0u8; 128];
                    let num1: u128 = split.next().unwrap().parse().unwrap();
                    let num2: u128 = split.next().unwrap().parse().unwrap();
                    let num3: u128 = split.next().unwrap().parse().unwrap();
                    let num4: u128 = split.next().unwrap().parse().unwrap();
                    let num5: u128 = split.next().unwrap().parse().unwrap();
                    let num6: u128 = split.next().unwrap().parse().unwrap();
                    let num7: u128 = split.next().unwrap().parse().unwrap();
                    let num8: u128 = split.next().unwrap().parse().unwrap();

                    let buf1 = num1.to_be_bytes();
                    let buf2 = num2.to_be_bytes();
                    let buf3 = num3.to_be_bytes();
                    let buf4 = num4.to_be_bytes();
                    let buf5 = num5.to_be_bytes();
                    let buf6 = num6.to_be_bytes();
                    let buf7 = num7.to_be_bytes();
                    let buf8 = num8.to_be_bytes();

                    buf[0..16].copy_from_slice(&buf1);
                    buf[16..32].copy_from_slice(&buf2);
                    buf[32..48].copy_from_slice(&buf3);
                    buf[48..64].copy_from_slice(&buf4);
                    buf[64..80].copy_from_slice(&buf5);
                    buf[80..96].copy_from_slice(&buf6);
                    buf[96..112].copy_from_slice(&buf7);
                    buf[112..128].copy_from_slice(&buf8);
                    let hint = postcard::from_bytes(&buf).unwrap();
                    let bento_clone = Arc::clone(&bento_sched);
                    let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                        bento_clone.parse_hint(hint);
                    }).unwrap();
                    //bento_sched.parse_hint(hint);
                },
                //Some("dequeue:") => {},
                _ => {},
            }

        }
    }
}

