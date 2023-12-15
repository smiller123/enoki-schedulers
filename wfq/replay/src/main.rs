#![feature(map_first_last)]
#![feature(hash_drain_filter)]
#![feature(thread_is_running)]
#![feature(let_chains)]

extern crate alloc;
extern crate scheduler_utils;
extern crate core;
extern crate serde;

pub mod sched;

use std::env;

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
        let mut split = line.split_whitespace();
        match split.next() {
            Some("thread_kill") => {
                done_tx.send(true);
                return;
            },
            Some("pnt:") => {
                let kpid_str = split.next().unwrap();

                let cpu_str = split.nth(3).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                let is_curr_str = split.nth(1).unwrap();
                let is_curr: bool = is_curr_str[0..is_curr_str.len()-1].parse().unwrap();
                let curr_pid_str = split.nth(1).unwrap();
                let curr_pid: u64 = curr_pid_str[0..curr_pid_str.len()-1].parse().unwrap();
                let curr_run_str = split.nth(1).unwrap();
                let curr_run: u64 = curr_run_str[0..curr_run_str.len()-1].parse().unwrap();

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

                done_tx.send(true);
                let res = bento.pick_next_task(cpu, sched, curr_runtime, guard);
                pnt_responses.insert(cpu, res);
            },
            Some("pnt_ret:") => {
                let kpid_str = split.next().unwrap();

                let cpu_str = split.next().unwrap();
                let cpu: i32 = cpu_str.parse().unwrap();
                let response = pnt_responses.get(&cpu).unwrap();
                if response.is_none() {
                    let expected = split.next().unwrap();
                    let got = alloc::format!("{:?}", response);
                    if got != expected {
                        println!("Expected pick next task response {} got {}",
                                 expected, got);
                    }
                } else if let Some(res_val) = response {
                    let pid_str = split.nth(3).unwrap();
                    let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                    let cpu_str = split.nth(1).unwrap();
                    let cpu: u32 = cpu_str[0..cpu_str.len()].parse().unwrap();
                    let expected = Schedulable {
                        pid: pid,
                        cpu: cpu,
                    };

                    if pid != res_val.get_pid() || cpu != res_val.get_cpu() {
                        println!("Expected pick next task response {:?}, got {:?}",
                                 expected, res_val);
                    }
                }
                done_tx.send(true);
                if split.find(|&x| x == "Error").is_some() && let Some(sched) = *response  {
                    let guard = RQLockGuard {
                        random_data: PhantomData
                    };
                    bento.pnt_err(sched.get_cpu() as i32, sched.get_pid(), 2, Some(sched), guard);
                }
            },
            Some("pnt_err:") => {
                let kpid_str = split.next().unwrap();

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
                done_tx.send(true);
                bento.pnt_err(cpu, pid, err, None, guard)
            },
            Some("balance_err:") => {
                let kpid_str = split.next().unwrap();

                let cpu_str = split.nth(3).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                let pid_str = split.nth(1).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let err_str = split.nth(1).unwrap();
                let err: i32 = err_str[0..err_str.len()].parse().unwrap();

                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                done_tx.send(true);
                bento.balance_err(cpu, pid, err, None, guard)
            },
            Some("dead:") => {
                let kpid_str = split.next().unwrap();

                let pid_str = split.nth(3).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()].parse().unwrap();

                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                done_tx.send(true);
                bento.task_dead(pid, guard)
            },
            Some("blocked:") => {
                let kpid_str = split.next().unwrap();

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
                done_tx.send(true);
                bento.task_blocked(pid, runtime, cpu_seqnum, cpu, from_switchto, guard)
            },
            Some("wakeup:") => {
                let kpid_str = split.next().unwrap();

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
                done_tx.send(true);
                bento.task_wakeup(pid, agent_data, deferrable > 0,
                                    last_ran_cpu, wake_up_cpu, waker_cpu, sched, guard);
            },
            Some("new:") => {
                println!("split {:?}", split);
                let kpid_str = split.next().unwrap();

                let pid_str = split.nth(3).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let runtime_str = split.nth(3).unwrap();
                let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                let runnable_str = split.nth(1).unwrap();
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
                done_tx.send(true);
                bento.task_new(pid, 0, runtime, runnable, prio, sched, guard);
            },
            Some("preempt:") => {
                let kpid_str = split.next().unwrap();

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
                done_tx.send(true);
                bento.task_preempt(pid, runtime, cpu_seqnum,
                                     cpu, from_switchto, was_latched, sched, guard);
            },
            Some("yield:") => {
                let kpid_str = split.next().unwrap();

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
                done_tx.send(true);
                bento.task_yield(pid, runtime, cpu_seqnum,
                                     cpu, from_switchto, sched, guard);
            },
            Some("departed:") => {
                let kpid_str = split.next().unwrap();

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
                done_tx.send(true);
                bento.task_departed(pid, cpu_seqnum,
                                     cpu, from_switchto, was_current, guard);
            },
            Some("switchto:") => {
                let kpid_str = split.next().unwrap();

                let pid_str = split.nth(3).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let runtime_str = split.nth(1).unwrap();
                let runtime: u64 = runtime_str[0..runtime_str.len()-1].parse().unwrap();
                let cpu_seq_str = split.nth(1).unwrap();
                let cpu_seqnum: u64 = cpu_seq_str[0..cpu_seq_str.len()-1].parse().unwrap();
                let cpu_str = split.nth(1).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()].parse().unwrap();

                done_tx.send(true);
                bento.task_switchto(pid, runtime, cpu_seqnum, cpu);
            },
            Some("affinity:") => {
                let kpid_str = split.next().unwrap();

                let pid_str = split.nth(3).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let cpumask_str = split.nth(1).unwrap();
                let cpumask: u64 = cpumask_str[0..cpumask_str.len()].parse().unwrap();

                done_tx.send(true);
                bento.task_affinity_changed(pid, cpumask);
            },
            Some("latched:") => {
                let kpid_str = split.next().unwrap();

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

                done_tx.send(true);
                bento.task_latched(pid, commit_time, cpu_seqnum, cpu, latched_preempt);
            },
            Some("tick:") => {
                let kpid_str = split.next().unwrap();

                let cpu_str = split.nth(3).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                let queued_str = split.nth(1).unwrap();
                let queued: i32 = queued_str[0..queued_str.len()].parse().unwrap();

                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                done_tx.send(true);
                bento.task_tick(cpu, queued != 0, guard);
            },
            Some("not_idle:") => {
                let kpid_str = split.next().unwrap();

                let cpu_str = split.nth(3).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();
                let pid_str = split.nth(1).unwrap();
                let next_pid: u64 = pid_str[0..pid_str.len()].parse().unwrap();
                done_tx.send(true);
                bento.cpu_not_idle(cpu, next_pid);
            },
            Some("select_rq:") => {
                let kpid_str = split.next().unwrap();

                let pid_str = split.nth(3).unwrap();
                let pid: u64 = pid_str[0..pid_str.len()-1].parse().unwrap();
                let waker_str = split.nth(1).unwrap();
                let waker: i32 = waker_str[0..waker_str.len()-1].parse().unwrap();
                let prev_str = split.nth(1).unwrap();
                let prev: i32 = prev_str[0..prev_str.len()-1].parse().unwrap();

                done_tx.send(true);
                select_responses.insert(pid, bento.select_task_rq(pid, waker, prev));
            },
            Some("select_rq_ret:") => {
                let kpid_str = split.next().unwrap();

                let pid_str = split.next().unwrap();
                let pid: u64 = pid_str.parse().unwrap();
                let response: i32 = *select_responses.get(&pid).unwrap();
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
                done_tx.send(true);
                bento.selected_task_rq(sched);
            },
            Some("migrate_rq:") => {
                let kpid_str = split.next().unwrap();

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
                done_tx.send(true);
                bento.migrate_task_rq(pid, sched, guard);
            },
            Some("balance:") => {
                let kpid_str = split.next().unwrap();

                let cpu_str = split.nth(3).unwrap();
                let cpu: i32 = cpu_str[0..cpu_str.len()-1].parse().unwrap();

                let guard = RQLockGuard {
                    random_data: PhantomData
                };
                done_tx.send(true);
                balance_responses.insert(cpu, bento.balance(cpu, guard));
            },
            Some("balance_ret:") => {
                let kpid_str = split.next().unwrap();

                let cpu_str = split.next().unwrap();
                let cpu: i32 = cpu_str.parse().unwrap();
                let response = balance_responses.get(&cpu).unwrap();
                let got = alloc::format!("{:?}", response);
                let expected = split.next().unwrap();
                if got != expected {
                    println!("Expected balance response {}, got {}",
                             expected, got);
                }
                done_tx.send(true);
            },
            Some("create_queue") => {
                let kpid_str = split.next().unwrap();

                if replay_name2.is_some() {
                    let fname = replay_name2.take().unwrap();
                    let buffer = RingBuffer::from_file(fname);
                    done_tx.send(true);
                    bento.register_queue(0, buffer);
                }
            },
            Some("create_reverse_queue") => {
                let kpid_str = split.next().unwrap();

                if replay_name2.is_some() {
                    let fname = replay_name2.take().unwrap();
                    let buffer = RingBuffer::from_file(fname);
                    done_tx.send(true);
                    bento.register_reverse_queue(0, buffer);
                }
            },
            Some("enter_queue:") => {
                let kpid_str = split.next().unwrap();

                let id_str = split.nth(3).unwrap();
                let id: i32 = id_str[0..id_str.len()-1].parse().unwrap();
                let entries_str = split.nth(1).unwrap();
                let entries: u32 = entries_str[0..entries_str.len()].parse().unwrap();
                done_tx.send(true);
                bento.enter_queue(id, entries);
            },
            Some("unregister_queue:") => {
                let kpid_str = split.next().unwrap();

                let id_str = split.nth(3).unwrap();
                let id: i32 = id_str[0..id_str.len()].parse().unwrap();

                done_tx.send(true);
                bento.unregister_queue(id);
            },
            Some("unregister_reverse_queue:") => {
                let kpid_str = split.next().unwrap();

                let id_str = split.nth(3).unwrap();
                let id: i32 = id_str[0..id_str.len()].parse().unwrap();

                done_tx.send(true);
                bento.unregister_rev_queue(id);
            },
            Some("dequeue2:") => {
                let kpid_str = split.next().unwrap();

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
                done_tx.send(true);
                bento.parse_hint(hint);
            },
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

    let mut lock_lines = BTreeMap::new();
    let mut found_locks = VecDeque::new();
    let mut found_threads = BTreeSet::new();
    let lines = read_lines(replay_name.clone()).unwrap();
    for line_res in lines {
        if let Ok(line) = line_res {
            let line_split = line.clone();
            let mut split = line_split.split_whitespace();
            let command = split.next().unwrap();
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
                found_threads.insert(String::from(kpid_str));
            }
        }
    }
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
        };
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
    };
    cpu_state.insert(u32::MAX, RwLock::new(state));

    let bento_sched = Arc::new(BentoSched {
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
        let replay_name3 = replay_name2.clone();
        let handle = thread::Builder::new().name(String::from(kpid)).spawn(move || {
            thread_func(&bento_clone, replay_name3, msg_rx, done_tx);
        }).unwrap();
        thread_handles.insert(kpid1, handle);
        msg_tx_queues.insert(kpid2, msg_tx);
        done_rx_queues.insert(kpid3, done_rx);
    }
    for line_res in lines {
        if let Ok(line) = line_res {
            if line.contains("_lock") || line.contains("lock_new") {
                continue;
            }
            let line_split = line.clone();
            let mut split = line_split.split_whitespace();
            let kpid_str = split.nth(1).unwrap();
            let kpid = String::from(kpid_str);
            let msg_tx_queue = msg_tx_queues.get(&kpid).unwrap();
            msg_tx_queue.send(line);
            let done_rx_queue = done_rx_queues.get(&kpid).unwrap();
            done_rx_queue.recv();
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

