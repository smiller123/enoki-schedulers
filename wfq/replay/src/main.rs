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

#![feature(map_first_last)]
#![feature(hash_drain_filter)]
#![feature(thread_is_running)]
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
use sched::CpuState;
use scheduler_utils::BentoScheduler;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use scheduler_utils::spin_rs::{register_lines, RwLock};

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::thread;
use std::sync::Arc;

use scheduler_utils::ringbuffer::RingBuffer;
use scheduler_utils::Schedulable;
use scheduler_utils::RQLockGuard;

use core::marker::PhantomData;
use thread::JoinHandle;

use std::sync::mpsc;

use std::time::Instant;
use std::time::Duration;

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}


fn thread_func(bento: &BentoSched, mut replay_name2: Option<String>, msg_rx: mpsc::Receiver<String>, done_tx: mpsc::Sender<bool>) {
    let mut pnt_responses = HashMap::new();
    let mut balance_responses = HashMap::new();
    let mut select_responses = HashMap::new();
    loop {
        let line = msg_rx.recv().unwrap();
    // while get lines
        let mut split = line.split_whitespace();
        // First argument will be at pos 3 after the next(),
        // then each argument is every 2 after that
        match split.next() {
            Some("thread_kill") => {
                done_tx.send(true);
                return;
            },
            Some("pnt:") => {
                //let start_parsed = Instant::now();
                //start_parsing = now.elapsed();
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let cpu_str = split.nth(3).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                let is_curr_str = split.nth(1).unwrap();
                let is_curr: bool = is_curr_str[0..is_curr_str.len()-1].parse().unwrap();
                let curr_pid_str = split.nth(1).unwrap();
                let curr_pid: u64 = curr_pid_str[0..curr_pid_str.len()-1].parse().unwrap();
                let curr_run_str = split.nth(1).unwrap();
                let curr_run: u64 = curr_run_str[0..curr_run_str.len()-1].parse().unwrap();
                //parsing = start_parsed.elapsed();
                //parsing = now.elapsed();

                let curr_runtime = if is_curr {
                    Some(curr_run)
                } else {
                    None
                };

                let sched = if is_curr {
                    Some(Schedulable {
                        pid: curr_pid,
                        cpu: cpu as u32,
                    })
                } else {
                    None
                };
                let guard = RQLockGuard {
                    random_data: PhantomData
                };

                //joining = now.elapsed();
                //let now2 = Instant::now();
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //join_time = now.elapsed();
                //println!("kpid is {}", kpid);
                //let bento_clone = Arc::clone(&bento_sched);
                //cloned = now.elapsed();
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                //println!("calling pnt");
                done_tx.send(true);
                //println!("calling pnt 1");
                let res = bento.pick_next_task(cpu, sched, curr_runtime, guard);
                //println!("calling pnt 2");
                pnt_responses.insert(cpu, res);
                //println!("calling pnt 3");
                //}).unwrap();
                //created = now.elapsed();
                //handles.insert(kpid, handle);
                //dont_remove_hands.insert(kpid);
                //pnt_response_hands.insert(cpu, handle);
            },
            Some("pnt_ret:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let cpu_str = split.next().unwrap();
                let cpu: i32 = cpu_str.parse().unwrap();
                //if pnt_response_hands.get(&cpu).is_none() {
                ////if handles.get(&kpid).is_none() {
                //    println!("Found pick next task response with no corresponding call");
                //}
                //let handle = pnt_response_hands.remove(&cpu).unwrap();
                //let handle = handles.remove(&kpid).unwrap();
                //dont_remove_hands.remove(&kpid);
                //println!("joining pnt ret");
                //let now2 = Instant::now();
                //let response = handle.join().unwrap();
                //join_time = now2.elapsed();
                //println!("joined pnt ret");
                //println!("looking for pnt_ret {}", cpu);
                let response = pnt_responses.get(&cpu).unwrap();
                if response.is_none() {
                    let expected = split.next().unwrap();
                    let got = alloc::format!("{:?}", response);
                    if got != expected {
                        println!("Expected pick next task response {} got {}",
                                 expected, got);
                    }
                } else if let Some(res_val) = response {
                    //let got = alloc::format!("{:?}", response);

                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let cpu_str = split.nth(1).unwrap();
                    let cpu: u32 = cpu_str[0..cpu_str.len()].parse().unwrap();
                    let expected = Schedulable {
                        pid: pid,
                        cpu: cpu,
                    };

                    //let expected = split.next().unwrap();
                    if pid != res_val.get_pid() || cpu != res_val.get_cpu() {
                    //if got != expected {
                        println!("Expected pick next task response {:?}, got {:?}",
                                 expected, res_val);
                    }
                }
                done_tx.send(true);
                //println!("pnt got {}, expected {}", got, expected);
                if split.find(|&x| x == "Error").is_some() && let Some(sched) = *response  {
                    let guard = RQLockGuard {
                        random_data: PhantomData
                    };
                    bento.pnt_err(sched.get_cpu() as i32, sched.get_pid(), 2, Some(sched), guard);
                }
            },
            Some("pnt_err:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let cpu_str = split.nth(3).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                let pid_str = split.nth(1).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let err_str = split.nth(1).unwrap();
                let err: i32 = err_str[0..err_str.len()].parse().unwrap();

                let sched = Schedulable {
                    pid: pid,
                    cpu: cpu as u32,
                };
                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.pnt_err(cpu, pid, err, None, guard)
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_blocked(pid, runtime, cpu_seqnum, cpu, from_switchto, sched);
            },
            Some("balance_err:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let cpu_str = split.nth(3).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                let pid_str = split.nth(1).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let err_str = split.nth(1).unwrap();
                let err: i32 = err_str[0..err_str.len()].parse().unwrap();

                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.balance_err(cpu, pid, err, None, guard)
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_blocked(pid, runtime, cpu_seqnum, cpu, from_switchto, sched);
            },
            Some("dead:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let pid_str = split.nth(3).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()].parse().unwrap();

                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.task_dead(pid, guard)
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_dead(pid);
            },
            Some("blocked:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

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

                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //let now2 = Instant::now();
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                ////join_time = now2.elapsed();
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.task_blocked(pid, runtime, cpu_seqnum, cpu, from_switchto, guard)
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_blocked(pid, runtime, cpu_seqnum, cpu, from_switchto, sched);
            },
            Some("wakeup:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

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
                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //let now2 = Instant::now();
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                ////join_time = now2.elapsed();
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.task_wakeup(pid, agent_data, deferrable > 0,
                                    last_ran_cpu, wake_up_cpu, waker_cpu, sched, guard);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_wakeup(pid, agent_data, deferrable > 0,
                 //                       last_ran_cpu, wake_up_cpu, waker_cpu, sched);
            },
            Some("new:") => {
                println!("split {:?}", split);
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let pid_str = split.nth(3).unwrap();
                //println!("{}", pid_str);
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let runtime_str = split.nth(3).unwrap();
                //println!("{}", runtime_str);
                let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                let runnable_str = split.nth(1).unwrap();
                //println!("{}", runnable_str);
                let runnable: u16 = runnable_str[0..runnable_str.len()-1].parse().unwrap();
                let prio_str = split.nth(1).unwrap();
                let prio: i32 = prio_str[0..prio_str.len()-1].parse().unwrap();
                let wakeup_str = split.nth(1).unwrap();
                let wakeup_cpu: i32 = wakeup_str[0..wakeup_str.len()-1].parse().unwrap();

                let sched = Schedulable {
                    pid: pid,
                    cpu: if wakeup_cpu == -1 {
                        u32::MAX
                    } else {
                        wakeup_cpu as u32
                    },
                };
                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //let now2 = Instant::now();
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                ////join_time = now2.elapsed();
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.task_new(pid, runtime, runnable, prio, sched, guard);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_new(pid, runtime, runnable, sched);
            },
            Some("preempt:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let pid_str = split.nth(3).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let runtime_str = split.nth(1).unwrap();
                let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                let cpu_seq_str = split.nth(1).unwrap();
                let cpu_seqnum: u64 = cpu_seq_str[0..cpu_seq_str.len()-1].parse().unwrap();
                let _agent_data_str = split.nth(1).unwrap();
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
                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //let now2 = Instant::now();
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                ////join_time = now2.elapsed();
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.task_preempt(pid, runtime, cpu_seqnum,
                                     cpu, from_switchto, was_latched, sched, guard);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_preempt(pid, runtime, cpu_seqnum,
                 //                        cpu, from_switchto, was_latched, sched);
            },
            Some("yield:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

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
                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //let now2 = Instant::now();
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                ////join_time = now2.elapsed();
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.task_yield(pid, runtime, cpu_seqnum,
                                     cpu, from_switchto, sched, guard);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_yield(pid, runtime, cpu_seqnum,
                 //                        cpu, from_switchto);
            },
            Some("departed:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

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

                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.task_departed(pid, cpu_seqnum,
                                     cpu, from_switchto, was_current, guard);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_departed(pid, cpu_seqnum,
                 //                        cpu, from_switchto, was_current);
            },
            Some("switchto:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let pid_str = split.nth(3).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let runtime_str = split.nth(1).unwrap();
                let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                let cpu_seq_str = split.nth(1).unwrap();
                let cpu_seqnum: u64 = cpu_seq_str[0..cpu_seq_str.len()-1].parse().unwrap();
                let cpu_str = split.nth(1).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()].parse().unwrap();

                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.task_switchto(pid, runtime, cpu_seqnum, cpu);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_switchto(pid, runtime, cpu_seqnum, cpu);
            },
            Some("affinity:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let pid_str = split.nth(3).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let cpumask_str = split.nth(1).unwrap();
                let cpumask: u64 = cpumask_str[0..cpumask_str.len()].parse().unwrap();

                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.task_affinity_changed(pid, cpumask);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_affinity_changed(pid);
            },
            Some("latched:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

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

                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.task_latched(pid, commit_time, cpu_seqnum, cpu, latched_preempt);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.task_latched(pid, commit_time, cpu_seqnum, cpu, latched_preempt);
            },
            Some("tick:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let cpu_str = split.nth(3).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                let queued_str = split.nth(1).unwrap();
                let queued: i32 = queued_str[0..queued_str.len()].parse().unwrap();

                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.task_tick(cpu, queued != 0, guard);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.cpu_tick(cpu);
            },
            Some("not_idle:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let cpu_str = split.nth(3).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                let pid_str = split.nth(1).unwrap();
                let next_pid: u64 = pid_str[0..pid_str.len()].parse().unwrap();
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.cpu_not_idle(cpu, next_pid);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.cpu_not_idle(cpu, next_pid);
            },
            Some("select_rq:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let pid_str = split.nth(3).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let waker_str = split.nth(1).unwrap();
                let waker: i32 = waker_str[0..waker_str.len()-1].parse().unwrap();
                let prev_str = split.nth(1).unwrap();
                let prev: i32 = prev_str[0..prev_str.len()-1].parse().unwrap();

                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                select_responses.insert(pid, bento.select_task_rq(pid, waker, prev));
                //}).unwrap();
                //select_response_hands.insert(pid, handle);
                //handles.insert(kpid, handle);
                //dont_remove_hands.insert(kpid);
                //let cpu = bento_sched.select_task_rq(pid);
                //select_response.insert(pid, cpu);
            },
            Some("select_rq_ret:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let pid_str = split.next().unwrap();
                let pid: u64 = pid_str.parse().unwrap();
                //if select_response_hands.get(&pid).is_none() {
                ////if handles.get(&kpid).is_none() {
                //    println!("Found select task rq response with no corresponding call");
                //}
                //let handle = select_response_hands.remove(&pid).unwrap();
                //let handle = handles.remove(&kpid).unwrap();
                //dont_remove_hands.remove(&kpid);
                //let response = handle.join().unwrap();
                let response: i32 = *select_responses.get(&pid).unwrap();
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
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.selected_task_rq(sched);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.selected_task_rq(sched);
            },
            Some("migrate_rq:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let pid_str = split.nth(3).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let next_str = split.nth(1).unwrap();
                let next_cpu: i32 = next_str[0..next_str.len()].parse().unwrap();

                let sched = Schedulable {
                    pid: pid,
                    cpu: next_cpu as u32,
                };
                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.migrate_task_rq(pid, sched, guard);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.migrate_task_rq(pid, sched);
            },
            Some("balance:") => {
                //let start_parsed = Instant::now();
                //start_parsing = now.elapsed();
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let cpu_str = split.nth(3).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                //parsing = now.elapsed();

                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                //let now2 = Instant::now();
               // joining = now.elapsed();
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //join_time = now.elapsed();
                //let bento_clone = Arc::clone(&bento_sched);
                //cloned = now.elapsed();
                //println!("balance thread name {}", kpid);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                balance_responses.insert(cpu, bento.balance(cpu, guard));
                //println!("ran balance");
                //}).unwrap();
                //created = now.elapsed();
                //let res = bento_sched.balance(cpu);
                //balance_response_hands.insert(cpu, handle);
                //handles.insert(kpid, handle);
                //dont_remove_hands.insert(kpid);
                //balance_response.insert(cpu, res);
            },
            Some("balance_ret:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let cpu_str = split.next().unwrap();
                let cpu: i32 = cpu_str.parse().unwrap();
                //if balance_response_hands.get(&cpu).is_none() {
                ////if handles.get(&kpid).is_none() {
                //    println!("Found balance response with no corresponding call");
                //}
                //let handle = balance_response_hands.remove(&cpu).unwrap();
                //let handle = handles.remove(&kpid).unwrap();
                //dont_remove_hands.remove(&kpid);
                //let now2 = Instant::now();
                //let response = handle.join().unwrap();
                let response = balance_responses.get(&cpu).unwrap();
                //join_time = now2.elapsed();
                //let response = balance_response.get(&cpu).unwrap();
                let got = alloc::format!("{:?}", response);
                let expected = split.next().unwrap();
                if got != expected {
                    println!("Expected balance response {}, got {}",
                             expected, got);
                }
                done_tx.send(true);
                //balance_response.remove(&cpu);
            },
            Some("create_queue") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                if replay_name2.is_some() {
                    //println!("registering queue");
                    let fname = replay_name2.take().unwrap();
                    let buffer = RingBuffer::from_file(fname);
                    //let bento_clone = Arc::clone(&bento_sched);
                    //if let Some(old_handle) = handles.remove(&kpid) {
                    //    old_handle.join();
                    //}
                    //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                    done_tx.send(true);
                    bento.register_queue(buffer);
                    //}).unwrap();
                    //handles.insert(kpid, handle);
                    //bento_sched.register_queue(buffer);
                }
            },
            Some("create_reverse_queue") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                if replay_name2.is_some() {
                    //println!("registering reverse queue");
                    let fname = replay_name2.take().unwrap();
                    let buffer = RingBuffer::from_file(fname);
                    //let bento_clone = Arc::clone(&bento_sched);
                    //if let Some(old_handle) = handles.remove(&kpid) {
                    //    old_handle.join();
                    //}
                    //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                    done_tx.send(true);
                    bento.register_reverse_queue(buffer);
                  //  }).unwrap();
                    //handles.insert(kpid, handle);
                    //bento_sched.register_reverse_queue(buffer);
                }
            },
            Some("enter_queue:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let id_str = split.nth(3).unwrap();
                let id: i32 = id_str[0..id_str.len()-1].parse().unwrap();
                let entries_str = split.nth(1).unwrap();
                let entries: u32 = entries_str[0..entries_str.len()].parse().unwrap();
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.enter_queue(id, entries);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.enter_queue(entries);
            },
            Some("unregister_queue:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let id_str = split.nth(3).unwrap();
                let id: i32 = id_str[0..id_str.len()].parse().unwrap();

                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.unregister_queue(id);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.unregister_queue();
            },
            Some("unregister_reverse_queue:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

                let id_str = split.nth(3).unwrap();
                let id: i32 = id_str[0..id_str.len()].parse().unwrap();

                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.unregister_rev_queue(id);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.unregister_rev_queue();
            },
            Some("dequeue2:") => {
                let kpid_str = split.next().unwrap();
                //let kpid: u32 = kpid_str.parse().unwrap();

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
                //if let Some(old_handle) = handles.remove(&kpid) {
                //    old_handle.join();
                //}
                //let bento_clone = Arc::clone(&bento_sched);
                //let handle = thread::Builder::new().name(kpid.to_string()).spawn(move || {
                done_tx.send(true);
                bento.parse_hint(hint);
                //}).unwrap();
                //handles.insert(kpid, handle);
                //bento_sched.parse_hint(hint);
            },
            //Some("dequeue:") => {},
            _ => {
                done_tx.send(true);
            },
        }
    }
}

fn main() {
    let replay_name_arg = env::args_os().nth(1).unwrap();
    let replay_name = replay_name_arg.into_string().unwrap();
    let mut replay_name2 = Some(replay_name.clone());
    let now = Instant::now();
    println!("replay name {}", replay_name);

    //let mut lock_lines = VecDeque::new();
    let mut lock_lines = BTreeMap::new();
    let mut found_locks = VecDeque::new();
    let mut found_threads = BTreeSet::new();
    //let mut found_lock = true;
    //while found_lock {
        //let mut lock_addr: Option<u64> = None;
        //let mut this_lock_lines = VecDeque::new();
        let lines = read_lines(replay_name.clone()).unwrap();
        //found_lock = false;
        for line_res in lines {
            if let Ok(line) = line_res {
                //println!("line is {}", line);
                let line_split = line.clone();
                let mut split = line_split.split_whitespace();
                let command = split.next().unwrap();
                //if lock_addr.is_none() && command == "lock_new:" {
                //    let this_lock = Some(split.nth(1).unwrap().parse().unwrap());
                //    if !found_locks.contains(&this_lock) {
                //        lock_addr = this_lock;
                //        found_locks.push_back(lock_addr);
                //        found_lock = true;
                //    }
                //}
                if command == "lock_new:" {
                    let this_lock = split.nth(1).unwrap().parse::<u64>().unwrap();
                    lock_lines.insert(this_lock, VecDeque::new());
                    found_locks.push_back(this_lock);
                } else if command.contains("_lock") {
                    let found_lock_addr = split.nth(1).unwrap().parse::<u64>().unwrap();
                    let found_lock_lines = lock_lines.get_mut(&found_lock_addr).unwrap();
                    found_lock_lines.push_back(line);
                } else {
                    let kpid_str = split.next().unwrap();
                    //let kpid: u32 = kpid_str.parse().unwrap();
                    found_threads.insert(String::from(kpid_str));
                }
            }
        }
        //println!("got lock {:?} num lines {}", lock_addr, this_lock_lines.len());
        //lock_lines.push_back(this_lock_lines);
    //}
    println!("got locks");
    println!("found threads {:?}", found_threads);
    let lines = read_lines(replay_name).unwrap();

    let mut all_lock_lines = VecDeque::new();
    for found in found_locks {
        let this_lock_lines = lock_lines.remove(&found).unwrap();
        all_lock_lines.push_back(this_lock_lines);
    }
    register_lines(all_lock_lines);



    let mut cpu_state = BTreeMap::new();
    for i in 0..8 {
        let state = CpuState {
            weight: 0,
            inv_weight: 0,
            curr: None,
            set: BTreeSet::new(),
            free_time: 0,
            load: 0,
            capacity: 0xff,
            should_report: 0
        };
        //let next_lock = found_locks.pop_front().unwrap();
        //let this_lock_lines = lock_lines.remove(&next_lock).unwrap();
        //cpu_state.insert(i, RwLock::new(state, lock_lines.pop_front().unwrap()));
        //cpu_state.insert(i, RwLock::new(state, this_lock_lines));
        cpu_state.insert(i, RwLock::new(state));
    }
    let state = CpuState {
        weight: 0,
        inv_weight: 0,
        curr: None,
        set: BTreeSet::new(),
        free_time: 0,
        load: 0,
        capacity: 0xff,
        should_report: 0
    };
    //let next_lock = found_locks.pop_front().unwrap();
    //let this_lock_lines = lock_lines.remove(&next_lock).unwrap();
    //cpu_state.insert(u32::MAX, RwLock::new(state, lock_lines.pop_front().unwrap()));
    //cpu_state.insert(u32::MAX, RwLock::new(state, this_lock_lines));
    cpu_state.insert(u32::MAX, RwLock::new(state));

    //let map_lock = found_locks.pop_front().unwrap();
    //let map_lines = lock_lines.remove(&map_lock).unwrap();
    //let state_lock = found_locks.pop_front().unwrap();
    //let state_lines = lock_lines.remove(&state_lock).unwrap();
    //let cpu_state_lock = found_locks.pop_front().unwrap();
    //let cpu_state_lines = lock_lines.remove(&cpu_state_lock).unwrap();
    //let user_q_lock = found_locks.pop_front().unwrap();
    //let user_q_lines = lock_lines.remove(&user_q_lock).unwrap();
    //let rev_q_lock = found_locks.pop_front().unwrap();
    //let rev_q_lines = lock_lines.remove(&rev_q_lock).unwrap();
    //let bal_lock = found_locks.pop_front().unwrap();
    //let bal_lines = lock_lines.remove(&bal_lock).unwrap();
    //let bal_cpu_lock = found_locks.pop_front().unwrap();
    //let bal_cpu_lines = lock_lines.remove(&bal_cpu_lock).unwrap();
    //let locked_lock = found_locks.pop_front().unwrap();
    //let locked_lines = lock_lines.remove(&locked_lock).unwrap();
    let bento_sched = Arc::new(BentoSched {
        //q: Some(RwLock::new(VecDeque::new(), lock_lines.pop_front().unwrap())),
        //map: Some(RwLock::new(BTreeMap::new(), map_lines)),
        //state: Some(RwLock::new(BTreeMap::new(), state_lines)),
        //cpu_state: Some(RwLock::new(cpu_state, cpu_state_lines)),
        //user_q: Some(RwLock::new(BTreeMap::new(), user_q_lines)),
        //rev_q: Some(RwLock::new(BTreeMap::new(), rev_q_lines)),
        //balancing: Some(RwLock::new(BTreeSet::new(), bal_lines)),
        //balancing_cpus: Some(RwLock::new(BTreeMap::new(), bal_cpu_lines)),
        //locked: Some(RwLock::new(BTreeSet::new(), locked_lines)),
        map: Some(RwLock::new(BTreeMap::new())),
        map2: Some(RwLock::new(BTreeMap::new())),
        state: Some(RwLock::new(BTreeMap::new())),
        state2: Some(RwLock::new(BTreeMap::new())),
        cpu_state: Some(RwLock::new(cpu_state)),
        user_q: Some(RwLock::new(BTreeMap::new())),
        rev_q: Some(RwLock::new(BTreeMap::new())),
        balancing: Some(RwLock::new(BTreeSet::new())),
        balancing_cpus: Some(RwLock::new(BTreeMap::new())),
        locked: Some(RwLock::new(BTreeSet::new())),
    });
    let preprocess = now.elapsed();
    //let bento_sched = Arc::new(BentoSched {
    //    //q: Some(RwLock::new(VecDeque::new(), lock_lines.pop_front().unwrap())),
    //    map: Some(RwLock::new(BTreeMap::new(), lock_lines.pop_front().unwrap())),
    //    state: Some(RwLock::new(BTreeMap::new(), lock_lines.pop_front().unwrap())),
    //    cpu_state: Some(RwLock::new(cpu_state, lock_lines.pop_front().unwrap())),
    //    user_q: Some(RwLock::new(BTreeMap::new(), lock_lines.pop_front().unwrap())),
    //    rev_q: Some(RwLock::new(BTreeMap::new(), lock_lines.pop_front().unwrap())),
    //    balancing: Some(RwLock::new(BTreeSet::new(), lock_lines.pop_front().unwrap())),
    //    balancing_cpus: Some(RwLock::new(BTreeMap::new(), lock_lines.pop_front().unwrap())),
    //    locked: Some(RwLock::new(BTreeSet::new(), lock_lines.pop_front().unwrap())),
    //});
    //let mut pnt_response_hands = HashMap::new();
    //let mut select_response_hands = HashMap::new();
    //let mut balance_response_hands = HashMap::new();
    //let mut handles: HashMap<u32, JoinHandle<()>> = HashMap::new();
    let mut thread_handles = HashMap::new();
    let mut msg_tx_queues = HashMap::new();
    let mut done_rx_queues = HashMap::new();
    for kpid in found_threads {
        let kpid1 = kpid.clone();
        let kpid2 = kpid.clone();
        let kpid3 = kpid.clone();
        let (msg_tx, msg_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let bento_clone = Arc::clone(&bento_sched);
        //cloned = now.elapsed();
        let replay_name3 = replay_name2.clone();
        let handle = thread::Builder::new().name(String::from(kpid)).spawn(move || {
            thread_func(&bento_clone, replay_name3, msg_rx, done_tx);
        }).unwrap();
        thread_handles.insert(kpid1, handle);
        msg_tx_queues.insert(kpid2, msg_tx);
        done_rx_queues.insert(kpid3, done_rx);
    }
    //let mut dont_remove_hands = HashSet::new();
    for line_res in lines {
        if let Ok(line) = line_res {
            //if line.contains("_lock") || line.contains("lock_new") {
            //    continue
            //}
            //println!("running {}", line);
            if line.contains("_lock") || line.contains("lock_new") {
                continue;
            }
            let line_split = line.clone();
            let mut split = line_split.split_whitespace();
            let kpid_str = split.nth(1).unwrap();
            let kpid = String::from(kpid_str);
            //let kpid: u32 = kpid_str.parse().unwrap();
            let msg_tx_queue = msg_tx_queues.get(&kpid).unwrap();
            msg_tx_queue.send(line);
            let done_rx_queue = done_rx_queues.get(&kpid).unwrap();
            done_rx_queue.recv();
            //println!("ran");
            //println!("running {}", line);
            //let mut parsing = Duration::new(0,0);
            //let mut start_parsing = Duration::new(0,0);
            //let mut created = Duration::new(0,0);
            //let mut joining = Duration::new(0,0);
            //let now = Instant::now();
            //let mut join_time = Duration::new(0,0);
            //let mut cloned = Duration::new(0,0);
            //handles.drain_filter(|k, v| !v.is_running());
            //handles.drain_filter(|k, v| v.is_finished());
            //let elapsed = now.elapsed();
            //println!("took {:?} start parsing {:?} parsing {:?} joining {:?} join time {:?} cloned {:?} created {:?}", elapsed, start_parsing, parsing, joining, join_time, cloned, created);

        }
    }
    let done = now.elapsed();
    println!("done");
    for kpid in msg_tx_queues.keys() {
        let line = String::from("thread_kill");
        let msg_tx_queue = msg_tx_queues.get(kpid).unwrap();
        msg_tx_queue.send(line);
        let handle = thread_handles.remove(kpid).unwrap();
        handle.join();
    }
    println!("preprocess {:?} end {:?}", preprocess, done);
}
