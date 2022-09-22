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

extern crate alloc;
extern crate scheduler_utils;
extern crate spin;
extern crate core;
extern crate serde;

pub mod enclave;
pub mod ghost;
pub mod sched;

use std::env;
use spin::RwLock;

use sched::BentoSched;
use scheduler_utils::BentoScheduler;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;
use std::collections::HashMap;

use scheduler_utils::ringbuffer::RingBuffer;

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
    //let replay_file = File::open(replay_name).unwrap();
    let lines = read_lines(replay_name).unwrap();

    let bento_sched = BentoSched {
        q: Some(RwLock::new(VecDeque::new())),
        map: Some(RwLock::new(BTreeMap::new())),
        user_q: Some(RwLock::new(None)),
    };
    let mut pnt_response = HashMap::new();
    let mut select_response = HashMap::new();
    let mut balance_response = HashMap::new();
    for line_res in lines {
        if let Ok(line) = line_res {
            let mut split = line.split_whitespace();
            // First argument will be at pos 3 after the next(),
            // then each argument is every 2 after that
            match split.next() {
                Some("pnt:") => {
                    let cpu_str = split.nth(3).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                    let res = bento_sched.pick_next_task(cpu);
                    pnt_response.insert(cpu, res);
                },
                Some("pnt_ret:") => {
                    let cpu_str = split.next().unwrap();
                    let cpu: i32 = cpu_str.parse().unwrap();
                    if pnt_response.get(&cpu).is_none() {
                        println!("Found pick next task response with no corresponding call");
                    }
                    let response = pnt_response.get(&cpu).unwrap();
                    let got = alloc::format!("{:?}", response);
                    let expected = split.next().unwrap();
                    if got != expected {
                        println!("Expected pick next task response {}, got {}",
                                 expected, got);
                    }
                    pnt_response.remove(&cpu);
                },
                Some("dead:") => {
                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()].parse().unwrap();
                    bento_sched.task_dead(pid);
                },
                Some("blocked:") => {
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
                    bento_sched.task_blocked(pid, runtime, cpu_seqnum, cpu, from_switchto);
                },
                Some("wakeup:") => {
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
                    bento_sched.task_wakeup(pid, agent_data, deferrable > 0,
                                            last_ran_cpu, wake_up_cpu, waker_cpu);
                },
                Some("new:") => {
                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let runtime_str = split.nth(1).unwrap();
                    println!("{}", runtime_str);
                    let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                    let runnable_str = split.nth(1).unwrap();
                    let runnable: u16 = runnable_str[0..runnable_str.len()-1].parse().unwrap();
                    bento_sched.task_new(pid, runtime, runnable);
                },
                Some("preempt:") => {
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
                    bento_sched.task_preempt(pid, runtime, cpu_seqnum,
                                             cpu, from_switchto, was_latched);
                },
                Some("yield:") => {
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
                    bento_sched.task_yield(pid, runtime, cpu_seqnum,
                                             cpu, from_switchto);
                },
                Some("departed:") => {
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
                    bento_sched.task_departed(pid, cpu_seqnum,
                                             cpu, from_switchto, was_current);
                },
                Some("switchto:") => {
                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let runtime_str = split.nth(1).unwrap();
                    let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                    let cpu_seq_str = split.nth(1).unwrap();
                    let cpu_seqnum: u64 = cpu_seq_str[0..cpu_seq_str.len()-1].parse().unwrap();
                    let cpu_str = split.nth(1).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()].parse().unwrap();
                    bento_sched.task_switchto(pid, runtime, cpu_seqnum, cpu);
                },
                Some("affinity:") => {
                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()].parse().unwrap();
                    bento_sched.task_affinity_changed(pid);
                },
                Some("latched:") => {
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
                    bento_sched.task_latched(pid, commit_time, cpu_seqnum, cpu, latched_preempt);
                },
                Some("tick:") => {
                    let cpu_str = split.nth(3).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()].parse().unwrap();
                    bento_sched.cpu_tick(cpu);
                },
                Some("not_idle:") => {
                    let cpu_str = split.nth(3).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                    let pid_str = split.nth(1).unwrap();
                    let next_pid: u64 = pid_str[0..pid_str.len()].parse().unwrap();
                    bento_sched.cpu_not_idle(cpu, next_pid);
                },
                Some("select_rq:") => {
                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let cpu = bento_sched.select_task_rq(pid);
                    select_response.insert(pid, cpu);
                },
                Some("select_rq_ret:") => {
                    let pid_str = split.next().unwrap();
                    let pid: u64 = pid_str.parse().unwrap();
                    if select_response.get(&pid).is_none() {
                        println!("Found select task rq response with no corresponding call");
                    }
                    let response = select_response.get(&pid).unwrap();
                    let got = alloc::format!("{}", response);
                    let expected = split.next().unwrap();
                    if got != expected {
                        println!("Expected select task rq response {}, got {}",
                                 expected, got);
                    }
                    select_response.remove(&pid);
                },
                Some("migrate_rq:") => {
                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let next_str = split.nth(1).unwrap();
                    let next_cpu: i32 = next_str[0..next_str.len()].parse().unwrap();
                    bento_sched.migrate_task_rq(pid, next_cpu);
                },
                Some("balance:") => {
                    let cpu_str = split.nth(3).unwrap();
                    let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                    let res = bento_sched.balance(cpu);
                    balance_response.insert(cpu, res);
                },
                Some("balance_ret:") => {
                    let cpu_str = split.next().unwrap();
                    let cpu: i32 = cpu_str.parse().unwrap();
                    if balance_response.get(&cpu).is_none() {
                        println!("Found balance response with no corresponding call");
                    }
                    let response = balance_response.get(&cpu).unwrap();
                    let got = alloc::format!("{:?}", response);
                    let expected = split.next().unwrap();
                    if got != expected {
                        println!("Expected balance response {}, got {}",
                                 expected, got);
                    }
                    balance_response.remove(&cpu);
                },
                Some("create_queue") => {
                    if replay_name2.is_some() {
                        println!("registering queue");
                        let fname = replay_name2.take().unwrap();
                        let buffer = RingBuffer::from_file(fname);
                        bento_sched.register_queue(buffer);
                    }
                },
                Some("enter_queue:") => {
                    let entries_str = split.nth(3).unwrap();
                    let entries: u32 = entries_str[0..entries_str.len()].parse().unwrap();
                    bento_sched.enter_queue(entries);
                },
                Some("unregister_queue:") => {
                    bento_sched.unregister_queue();
                },
                //Some("dequeue:") => {},
                _ => {},
            }

        }
    }
}

