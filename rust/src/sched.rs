#[cfg(not(feature = "replay"))]
use bento::println;
#[cfg(not(feature = "replay"))]
use bento::scheduler_utils;
#[cfg(not(feature = "replay"))]
use bento::spin_rs::RwLock;
#[cfg(feature = "replay")]
use scheduler_utils;
#[cfg(feature = "replay")]
use scheduler_utils::spin_rs::RwLock;
//use rbtree::RBTree;

use serde::{Serialize, Deserialize};

use self::scheduler_utils::*;


//use bento::bindings as c;
//use bento::kernel::ffi;
//use bento::kernel::raw;

//use bento::std::ffi::OsStr;
//use bento::kernel::kobj::CStr;

//use spin::RwLock;

//use bento::spin_rs;
//use bento::spin_rs::RwLock;

//use enclave::LocalEnclave;
//use ghost::Ghost;

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;
//use slice_rbtree::tree::{tree_size, RBTree, TreeParams};

use core::mem;
use core::str;
use core::fmt::Debug;

use RQLockGuard;
use self::ringbuffer::RingBuffer;
use self::sched_core::resched_cpu;
//use bento::kernel::time::Timespec64;
//use bento::kernel::time::getnstimeofday64_rs;
//use bento::kernel::time::diff_ns;
static REPORT: core::sync::atomic::AtomicI64 = core::sync::atomic::AtomicI64::new(0);

#[repr(C)]
pub struct BentoSched {
//    pub sets_list: Option<RwLock<BTreeMap<u32, RwLock<BTreeSet<(u64,u64)>>>>>,
    pub map: Option<RwLock<BTreeMap<u64, Schedulable>>>,
    pub state: Option<RwLock<BTreeMap<u64, ProcessState>>>,
    pub cpu_state: Option<RwLock<BTreeMap<u32, RwLock<CpuState>>>>,
    pub user_q: Option<RwLock<BTreeMap<i32, RingBuffer<UserMessage>>>>,
    pub rev_q: Option<RwLock<BTreeMap<i32, RingBuffer<UserMessage>>>>,
    pub balancing: Option<RwLock<BTreeSet<u64>>>,
    pub balancing_cpus: Option<RwLock<BTreeMap<u32, u32>>>,
    pub locked: Option<RwLock<BTreeSet<u64>>>,
}

pub struct ProcessState {
    prio: i32,
    vruntime: u64,
    last_runtime: u64,
    cpu: u32
}

//#[derive(Default)]
pub struct CpuState {
    pub weight: u64,
    pub inv_weight: u64,
    pub curr: Option<u64>,
    pub set: BTreeSet<(u64, u64)>,
    pub free_time: u64,
    pub load: u8,
    pub capacity: u8,
    pub should_report: u64,
}

pub struct UpgradeData {
    //pub cpu_state: Option<RwLock<BTreeMap<u32, RwLock<CpuState>>>>,
    //pub sets_list: Option<RwLock<BTreeMap<u32, RwLock<BTreeSet<(u64,u64)>>>>>,
    //map: Option<RwLock<BTreeMap<u64, Schedulable>>>
    map: Option<RwLock<BTreeMap<u64, Schedulable>>>,
    state: Option<RwLock<BTreeMap<u64, ProcessState>>>,
    cpu_state: Option<RwLock<BTreeMap<u32, RwLock<CpuState>>>>,
    user_q: Option<RwLock<BTreeMap<i32, RingBuffer<UserMessage>>>>,
    rev_q: Option<RwLock<BTreeMap<i32, RingBuffer<UserMessage>>>>,
    balancing: Option<RwLock<BTreeSet<u64>>>,
    balancing_cpus: Option<RwLock<BTreeMap<u32, u32>>>,
    locked: Option<RwLock<BTreeSet<u64>>>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct UserMessage {
    val: i32
}

fn calculate_vruntime(runtime: u64, prio: i32) -> u64 {
    let inv_weight = sched_core::SCHED_PRIO_TO_WMULT[(prio - 100) as usize] as u64;
    let mut factor = 1024 * inv_weight;
    let mut shift = 32;
    while factor >> 32 > 0 {
        factor >>= 1;
        shift -= 1;
    }
    let runtime_calc = runtime as u64 * factor;
    runtime_calc >> shift
}


impl BentoSched {
    //fn update_curr(&self, cpu: i32) {
    //    let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
    //    let mut cpu_state = all_cpu_state.get(&(cpu as u32)).unwrap().write();
    //    if let Some(curr) = cpu_state.curr {
    //        if cpu_state.exec_start > 0 {
    //            let now = unsafe {
    //                bento::bindings::cpu_clock_task(cpu)
    //            };
    //            let delta_runtime = now - cpu_state.exec_start;
    //            let mut state = self.state.as_ref().unwrap().write();
    //            if state.get_mut(&curr).is_none() {
    //                return;
    //            }
    //            let old_vruntime;
    //            let vruntime;
    //            {
    //                let proc_state = state.get_mut(&curr).unwrap();
    //                old_vruntime = proc_state.vruntime;
    //                proc_state.vruntime = old_vruntime + calculate_vruntime(delta_runtime, proc_state.prio);
    //                vruntime = proc_state.vruntime;
    //                let old_last = proc_state.last_runtime;
    //                proc_state.last_runtime += delta_runtime;
    //                //if cpu == 0 {
    //                //println!("updating {} old runtime {} new runtime {} on cpu {}", curr, old_vruntime, proc_state.vruntime, cpu);
    //                //}
    //            }
    //            //println!("updating {} on {}", curr, cpu);
    //            let remove_val = (old_vruntime, curr);
    //            let mut real_set = &mut cpu_state.set;
    //            //let found = real_set.get(&remove_val);
    //            //println!("remove was in? {:?}", found);
    //            real_set.remove(&remove_val);
    //            //real_set.insert(remove_val);
    //            //real_set.insert((vruntime, curr));
    //            cpu_state.exec_start = unsafe {
    //                bento::bindings::cpu_clock_task(cpu)
    //            };
    //        }
    //    } else {
    //        cpu_state.free_time += 1;
    //        //println!("curr empty for cpu {}", cpu);
    //    }
    //}

    fn add_weight(&self, cpu_state: &mut CpuState, prio: i32) {
        //let mut cpu_state = self.cpu_state.as_ref().unwrap().write();
        //let curr_weight_opt = cpu_state.get_mut(&cpu);
        let weight = sched_core::SCHED_PRIO_TO_WEIGHT[(prio - 100) as usize] as u64;
        //if curr_weight_opt.is_some() {
            //println!("got weight {:?}", curr_weight_opt);
  //          let curr_weight = curr_weight_opt.unwrap();
            cpu_state.weight += weight;
            let weight_shift = cpu_state.weight >> 10;
            if weight_shift > 0 {
                let inv_weight = 0xffffffff / weight_shift;
                cpu_state.inv_weight = inv_weight;
            } else {
                cpu_state.inv_weight = 0xffffffff;
            }
            //let inv_weight = 0xffffffff / weight_shift;
            //cpu_state.inv_weight = inv_weight;
            //println!("adding weight {} total weight {}", weight, cpu_state.weight);
           // println!("putting cpu {} {} {}", curr_weight.weight, inv_weight);
        //} else {
        //    let weight_shift = weight >> 10;
        //    let inv_weight = 0xffffffff / weight_shift;
        //    let new_weight = CpuState {
        //        weight: weight,
        //        inv_weight: inv_weight
        //    };
        //    println!("putting cpu {} {} {}", cpu, new_weight.weight, inv_weight);
        //    cpu_state.insert(cpu, new_weight);
        //}
    }

    fn remove_weight(&self, cpu_state: &mut CpuState, prio: i32) {
        let weight = sched_core::SCHED_PRIO_TO_WEIGHT[(prio - 100) as usize] as u64;
        if cpu_state.weight >= weight {
            cpu_state.weight -= weight;
        } else {
            cpu_state.weight = 0;
        }
        let weight_shift = cpu_state.weight >> 10;
        if weight_shift > 0 {
            let inv_weight = 0xffffffff / weight_shift;
            cpu_state.inv_weight = inv_weight;
        } else {
            cpu_state.inv_weight = 0xffffffff;
        }
        //println!("removing weight {} total weight {}", weight, cpu_state.weight);
    }
}

impl BentoScheduler<'_, '_, UpgradeData, UpgradeData, UserMessage, UserMessage> for BentoSched {
    fn get_policy(&self) -> i32 {
        10
    }

    //fn init(&mut self) {
   // }

    fn task_new(&self, pid: u64, runtime: u64, runnable: u16, prio: i32, sched: Schedulable, _guard: RQLockGuard) {
        //if sched.get_pid() == 0 {
        //println!("task new pid {} prio {} runtime {} cpu {}", pid, prio, runtime, sched.get_cpu());
       // }
        let mut vruntime = calculate_vruntime(runtime, prio);
        if runnable > 0 {
            let cpu;
            {
                let mut map = self.map.as_ref().unwrap().write();
                if map.get(&pid).is_none() {
                    map.insert(pid, sched);
                }
                cpu = map.get(&pid).unwrap().get_cpu();
            }
            //println!("cpu is {}", cpu);
            //let sets_list = self.sets_list.as_ref().unwrap().read();
            //let cpu_state
            {
                let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
                let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
                //let mut real_set = cpu_state.set.get(&cpu).unwrap().write();
                let mut real_set = &mut cpu_state.set;
                if let Some((min_vruntime, _)) = real_set.first() {
                    //if *min_vruntime > vruntime {
                    //    println!("new updating vruntime to {}", *min_vruntime);
                    //}
                    vruntime = core::cmp::max(vruntime, *min_vruntime);
                }
                real_set.insert((vruntime, pid));
                //println!("new pid {} cpu {} set is {:?}", pid, sched.get_cpu(), real_set);
                //println!("new pid {} cpu {} curr is {:?}", pid, sched.get_cpu(), cpu_state.curr);
                self.add_weight(&mut cpu_state, prio);
            }
        //{
            let mut state = self.state.as_ref().unwrap().write();
            let procstate = ProcessState {
                prio: prio,
                vruntime: vruntime,
                last_runtime: runtime,
                cpu: cpu,
            };
            state.insert(pid, procstate);
        //}
            //let mut cpu_state = self.cpu_state.as_ref().unwrap().write();
            //let curr_weight_opt = cpu_state.get_mut(&cpu);
            //let weight = sched_core::SCHED_PRIO_TO_WEIGHT[(prio - 100) as usize] as u64;
            //if curr_weight_opt.is_some() {
            //    //println!("got weight {:?}", curr_weight_opt);
            //    let curr_weight = curr_weight_opt.unwrap();
            //    curr_weight.weight += weight;
            //    let weight_shift = curr_weight.weight >> 10;
            //    let inv_weight = 0xffffffff / weight_shift;
            //    curr_weight.inv_weight = inv_weight;
            //    println!("putting cpu {} {} {}", cpu, curr_weight.weight, inv_weight);
            //} else {
            //    let weight_shift = weight >> 10;
            //    let inv_weight = 0xffffffff / weight_shift;
            //    let new_weight = CpuState {
            //        weight: weight,
            //        inv_weight: inv_weight
            //    };
            //    println!("putting cpu {} {} {}", cpu, new_weight.weight, inv_weight);
            //    cpu_state.insert(cpu, new_weight);
            //}
            //let total_weight = weight;
            //for (_, other_pid) in set.iter() {
            //    let other_state = state.get(&other_pid).unwrap();
            //    let other_prio = other_state.prio;
            //    let other_weight = sched_core::SCHED_PRIO_TO_WEIGHT[(other_prio - 100) as usize] as u64;
            //    total_weight += other_weight;
            //}
            //println!("added {} to tree", pid);
        }
    }

    fn task_prio_changed(&self, pid: u64, _prio: i32, _guard: RQLockGuard) {
        //println!("task prio changed {}", pid);
    }

    fn task_wakeup(&self, pid: u64, _agent_data: u64, deferrable: bool,
                   _last_run_cpu: i32, _wake_up_cpu: i32, _waker_cpu: i32,
                   sched: Schedulable, _guard: RQLockGuard) {
        //if sched.get_cpu() == 0 {
        //println!("wakeup {} {} cpu {}", pid, _wake_up_cpu, sched.get_cpu());
        //}
        let mut new_sched = false;
        let cpu = sched.get_cpu();
        {
            let mut map = self.map.as_ref().unwrap().write();
            if let Some(old_sched) = map.get(&pid) {
                if old_sched.get_cpu() != sched.get_cpu() {
                    new_sched = true;
                }
            } else {
                new_sched = true;
            }
            map.insert(pid, sched);
        }
        //let sets_list = self.sets_list.as_ref().unwrap().read();
        //let mut real_set = sets_list.get(&cpu).unwrap().write();
        //real_set.insert((proc_state.vruntime, pid));

        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
        if let Some(curr_pid) = cpu_state.curr {
            if curr_pid == pid {
                return;
                //println!("fucked situation on wakeup");
            }
        }
        //let mut real_set = cpu_state.set.get(&cpu).unwrap().write();
        let mut real_set = &mut cpu_state.set;
        let mut state = self.state.as_ref().unwrap().write();
        let proc_state = state.get_mut(&pid).unwrap();
        if let Some((min_vruntime, _)) = real_set.first() {
            //let old_vruntime = proc_state.vruntime;
            if *min_vruntime > 6000000 {
                proc_state.vruntime = core::cmp::max(*min_vruntime - 6000000, proc_state.vruntime);
                //if old_vruntime != proc_state.vruntime && sched.get_cpu() == 0 {
                //    println!("wakeup updating {} vruntime from {} to {}, min {}", pid, old_vruntime, proc_state.vruntime, *min_vruntime);
                //}
            }
        }
        proc_state.cpu = cpu;
        real_set.insert((proc_state.vruntime, pid));
        //println!("wakeup {} {} {}", pid, _wake_up_cpu, real_set.len());
        //self.add_weight(&mut cpu_state, prio);
        //let cpu_state = self.cpu_state.as_ref().unwrap().read();
        //if cpu_state.get(&(cpu as u32)).is_none() {
        //    println!("wakeup cpu is {}", cpu);
        //}
        //if new_sched {
            //println!("wakeup pid {} cpu {} set is {:?}", pid, sched.get_cpu(), real_set);
            //println!("wakeup pid {} cpu {} curr is {:?}", pid, sched.get_cpu(), cpu_state.curr);
            self.add_weight(&mut cpu_state, proc_state.prio);
            //if let Some(curr_pid) = cpu_state.curr {
            //    if curr_pid == pid {
            //        println!("fucked situation on wakeup");
            //    }
            //}
            //let mut cpu_state = self.cpu_state.as_ref().unwrap().write();
            //let curr_weight_opt = cpu_state.get_mut(&cpu);
            //let weight = sched_core::SCHED_PRIO_TO_WEIGHT[(proc_state.prio - 100) as usize] as u64;
            //if curr_weight_opt.is_some() {
            //    //println!("got weight {:?}", curr_weight_opt);
            //    let curr_weight = curr_weight_opt.unwrap();
            //    curr_weight.weight += weight;
            //    let weight_shift = curr_weight.weight >> 10;
            //    let inv_weight = 0xffffffff / weight_shift;
            //    curr_weight.inv_weight = inv_weight;
            //    println!("wakeup putting cpu {} {} {}", cpu, curr_weight.weight, inv_weight);
            //} else {
            //    let weight_shift = weight >> 10;
            //    let inv_weight = 0xffffffff / weight_shift;
            //    let new_weight = CpuState {
            //        weight: weight,
            //        inv_weight: inv_weight
            //    };
            //    println!("wakeup putting cpu {} {} {}", cpu, new_weight.weight, inv_weight);
            //    //println!("putting cpu {}", cpu);
            //    cpu_state.insert(cpu, new_weight);
            //}
        //}
        //let cpu_state = self.cpu_state.as_ref().unwrap().read();
        //if cpu_state.get(&(cpu as u32)).is_none() {
        //    println!("wakeup cpu is {}", cpu);
        //}
        //println!("added {} to tree", pid);
    }

    fn task_preempt(&self, pid: u64, runtime: u64, _cpu_seqnum: u64, _cpu: i32,
                    _from_switchto: i8, _was_latched: i8, sched: Schedulable, _guard: RQLockGuard) {
        let vruntime;
        let old_vruntime;
        let cpu = sched.get_cpu();
        {
            let mut map = self.map.as_ref().unwrap().write();
            map.insert(pid, sched);
        }
        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
        let mut real_set = &mut cpu_state.set;
        {
            let mut state = self.state.as_ref().unwrap().write();
            let procstate = state.get_mut(&pid).unwrap();
            old_vruntime = procstate.vruntime;
            let delta_runtime = runtime - procstate.last_runtime;
            //if let Some((min_vruntime, _)) = real_set.first() {
            //    if *min_vruntime > 6000000 {
            //        procstate.vruntime = core::cmp::max(*min_vruntime - 6000000, procstate.vruntime);
            //    }
            //}
            procstate.vruntime += calculate_vruntime(delta_runtime, procstate.prio);
            //procstate.vruntime = calculate_vruntime(runtime, procstate.prio);
            //let other_vruntime = calculate_vruntime(runtime, procstate.prio);
            //if other_vruntime != procstate.vruntime {
            //   println!("preempt {} old vruntime {} new {} runtime {} last runtime {}", pid, other_vruntime, procstate.vruntime, runtime, procstate.last_runtime);
            //}
            vruntime = procstate.vruntime;
            procstate.last_runtime = runtime;
            procstate.cpu = cpu;
        }
        //if sched.get_cpu() == 0 {
        //println!("preempt {} {} {} {}", pid, _cpu, cpu, vruntime);
        //}
        let remove_val = (old_vruntime, pid);
        //let mut map = self.map.as_ref().unwrap().write();
        //map.insert(pid, sched);
        //let cpu = sched.get_cpu();
        //let sets_list = self.sets_list.as_ref().unwrap().read();
        //let mut real_set = sets_list.get(&cpu).unwrap().write();
        //let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        //let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
        //let mut real_set = &mut cpu_state.set;
        real_set.remove(&remove_val);
        real_set.insert((vruntime, pid));
        if cpu_state.curr == Some(pid) {
            cpu_state.curr = None;
        }
        ////let cpu_state = self.cpu_state.as_ref().unwrap().read();
        //if cpu_state.get(&(cpu as u32)).is_none() {
        //    println!("preempt cpu is {}", cpu);
        //}
        //println!("added {} to tree", pid);
        //hrtick::hrtick_start(_cpu, 10000);
    }

    fn task_blocked(&self, pid: u64, runtime: u64, _cpu_seqnum: u64,
                    cpu: i32, _from_switchto: i8, _guard: RQLockGuard) {
        //let cpu = sched.get_cpu();
        //{
        //    //let mut map = self.map.as_ref().unwrap().write();
        //    //cpu = map.get(&pid).unwrap().get_cpu();
        //    //map.insert(pid, sched);
        //}
        let old_vruntime;
        let prio;
        let vruntime;
        {
            let mut state = self.state.as_ref().unwrap().write();
            let procstate = state.get_mut(&pid).unwrap();
            old_vruntime = procstate.vruntime;
            prio = procstate.prio;
            let delta_runtime = runtime - procstate.last_runtime;
            //procstate.vruntime += runtime;
            procstate.vruntime += calculate_vruntime(delta_runtime, procstate.prio);
            //procstate.vruntime = calculate_vruntime(runtime, procstate.prio);
            //let other_vruntime = calculate_vruntime(runtime, procstate.prio);
            //if other_vruntime != procstate.vruntime && runtime != procstate.vruntime {
            //    println!("blocked {} prio {} old vruntime {} new {} runtime {} last runtime {}", pid, procstate.prio, other_vruntime, procstate.vruntime, runtime, procstate.last_runtime);
            //    let inv_weight = sched_core::SCHED_PRIO_TO_WMULT[(procstate.prio - 100) as usize] as u64;
            //    let factor = 1024 * inv_weight;
            //    let runtime_calc = runtime as u64 * factor;
            //    let calc_vrun = runtime_calc >> 32;
            //    println!("inv {} fact {} calc {} vrun {}", inv_weight, factor, runtime_calc, calc_vrun);
            //}
            vruntime = procstate.vruntime;
            procstate.last_runtime = runtime;
            procstate.cpu = cpu as u32;
        }
        //if sched.get_cpu() == 0 {
        //println!("blocked {} {} {} {}", pid, _cpu, cpu, vruntime);
        //}
        let remove_val = (old_vruntime, pid);

        //let sets_list = self.sets_list.as_ref().unwrap().read();
        //let mut real_set = sets_list.get(&cpu).unwrap().write();
        //real_set.remove(&remove_val);
        
        {
            let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
            let mut cpu_state = all_cpu_state.get(&(cpu as u32)).unwrap().write();
            let mut real_set = &mut cpu_state.set;
            if real_set.remove(&remove_val) {
                //println!("blocked pid {} cpu {}, set is {:?}", pid, cpu, real_set);
                //println!("blocked pid {} cpu {} curr is {:?}", pid, cpu, cpu_state.curr);
                self.remove_weight(&mut cpu_state, prio);
            }
            if cpu_state.curr == Some(pid) {
                //println!("blocked curr pid {} cpu {} curr is {:?}", pid, cpu, cpu_state.curr);
                cpu_state.curr = None;
                self.remove_weight(&mut cpu_state, prio);
            }

        }
        //map.insert(pid, cpu as u32);
        //let new_sched = sched.get_cpu() != cpu;
        //if new_sched {
        //    let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        //    let mut cpu_state = all_cpu_state.get(&sched.get_cpu()).unwrap().write();
        //    self.add_weight(&mut cpu_state, prio);
        //}
        //let cpu_state = self.cpu_state.as_ref().unwrap().read();
        //if cpu_state.get(&(sched.get_cpu() as u32)).is_none() {
        //    println!("block cpu is {}", sched.get_cpu());
        //}
    }

    fn task_dead(&self, pid: u64, _guard: RQLockGuard) {
        //println!("deadge {}", pid);
        // TODO: Remove weight from cpu, remove from set, remove from balacing?, keep track of size
        // of sets
        let prio;
        let old_vruntime;
        {
            let mut state = self.state.as_ref().unwrap().write();
            let procstate = state.get_mut(&pid).unwrap();
            old_vruntime = procstate.vruntime;
            prio = procstate.prio;
        }
        let remove_val = (old_vruntime, pid);

        let cpu;
        {
            let mut map = self.map.as_ref().unwrap().write();
            cpu = map.get(&pid).unwrap().get_cpu();
            map.remove(&pid);
        }
        {
            let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
            let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
            let mut real_set = &mut cpu_state.set;
            if real_set.remove(&remove_val) {
                //println!("dead pid {} cpu {} set is {:?}", pid, cpu, real_set);
                //println!("dead pid {} cpu {} curr is {:?}", pid, cpu, cpu_state.curr);
                self.remove_weight(&mut cpu_state, prio);
            }
            if cpu_state.curr == Some(pid) {
                //println!("dead curr pid {} cpu {} curr is {:?}", pid, cpu, cpu_state.curr);
                cpu_state.curr = None;
                //println!("dead pid {} cpu {}", pid, cpu);
                self.remove_weight(&mut cpu_state, prio);
            }

        }
    }

    fn task_departed(
        &self,
        _pid: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _from_switchto: i8,
        _was_current: i8,
        _guard: RQLockGuard
    ) {
        println!("task departed called");
    }

    fn pick_next_task(&self, cpu: i32, curr_sched: Option<Schedulable>,
                      curr_runtime: Option<u64>, _guard: RQLockGuard) -> Option<Schedulable> {
        //println!("curr_sched {:?}", curr_sched);
        //return None;
        //if self.rev_q.is_some()
        //{
        //    let mut rev_q_guard = self.rev_q.as_ref().unwrap().write();
        //    if rev_q_guard.is_some() {
        //    let rev_q = rev_q_guard.as_mut().unwrap();
        //    let msg = UserMessage { val: 10 };
        //    rev_q.enqueue(msg);
        //    }
        //}
        {
        //let mut start = Timespec64::new();
        //let mut end = Timespec64::new();
        //let mut step1 = Timespec64::new();
        //let mut step11 = Timespec64::new();
        //let mut step12 = Timespec64::new();
        //let mut step2 = Timespec64::new();
        //let mut step21 = Timespec64::new();
        //let mut step22 = Timespec64::new();
        //let mut step23 = Timespec64::new();
        //let mut step24 = Timespec64::new();
        //let mut step3 = Timespec64::new();
        ////let mut step4 = Timespec64::new();
        //getnstimeofday64_rs(&mut start);
        //println!("pick next task called");
        //let mut tree = self.tree.as_ref().unwrap().write();

        //self.update_curr(cpu);
        let curr_pid_opt = if let Some(current) = curr_sched {
            let pid = current.get_pid();
            let mut map = self.map.as_ref().unwrap().write();
            map.insert(current.get_pid(), current);
            Some(pid)
        } else {
            None
        };
        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&(cpu as u32)).unwrap().write();
        //if let Some(runtime) = curr_runtime {
        let curr_reintro = if let Some(runtime) = curr_runtime &&
            let Some(curr_pid) = curr_pid_opt &&
            let Some(curr_pid2) = cpu_state.curr &&
            curr_pid == curr_pid2 {
            //let state = self.state.as_ref().unwrap().read();
            //let proc_state = state.get(&curr).unwrap();
            //Some((proc_state.vruntime, curr))
            let mut state = self.state.as_ref().unwrap().write();
            let procstate = state.get_mut(&curr_pid).unwrap();
            //old_vruntime = procstate.vruntime;
            let delta_runtime = runtime - procstate.last_runtime;
            //if let Some((min_vruntime, _)) = real_set.first() {
            //    if *min_vruntime > 6000000 {
            //        procstate.vruntime = core::cmp::max(*min_vruntime - 6000000, procstate.vruntime);
            //    }
            //}
            procstate.vruntime += calculate_vruntime(delta_runtime, procstate.prio);
            //procstate.vruntime = calculate_vruntime(runtime, procstate.prio);
            //let other_vruntime = calculate_vruntime(runtime, procstate.prio);
            //if other_vruntime != procstate.vruntime {
            //    println!("preempt {} old vruntime {} new {} runtime {} last runtime {}", pid, other_vruntime, procstate.vruntime, runtime, procstate.last_runtime);
            //}
            procstate.last_runtime = runtime;
            Some((procstate.vruntime, curr_pid))
        } else {
            None
        };
        let should_report = cpu_state.should_report;

        let set = &mut cpu_state.set;
        //let sets_list = self.sets_list.as_ref().unwrap().read();
        //let mut set = sets_list.get(&(cpu as u32)).unwrap().write();
        //let mut set = self.set.as_ref().unwrap().write();
        //if set.len() > 0 {
        if let Some(curr_val) = curr_reintro {
            //println!("reintroducing {:?}", curr_val);
            set.insert(curr_val);
        }
        //getnstimeofday64_rs(&mut step1);
        //if set.len() > 1 {
            //println!("cpu {} set len {}", cpu, set.len());
        //}
        //if set.len() == 0 {
        //    cpu_state.load = 0;
        //}
        //cpu_state.load = cpu_state.load << 1;
        //if set.len() > 1 {
        //    cpu_state.load = cpu_state.load & 1;
        //}
        //if !set.is_empty() {
            //println!("cpu {} picking from {:?}", cpu, set);
        //}
        if should_report % 10000 == 0 {
            //println!("cpu {} set {:?}", cpu, set);
        }
        if let Some((vruntime, pid)) = set.pop_first() {
        //if let Some((vruntime, pid)) = set.first() {
            //let mut set = &mut cpu_state.set;
            //getnstimeofday64_rs(&mut step11);
            let sched_opt = {
                //let map = self.map.as_ref().unwrap().read();
                let mut map = self.map.as_ref().unwrap().write();
                //map.get(&pid).and_then(|x| Some(*x))
                //map.get(&pid)
                map.remove(&pid)
            };
            //getnstimeofday64_rs(&mut step12);
            if sched_opt.is_none() {
                println!("Terrible problem, pid has no cpu");
                //set.insert((vruntime, pid));
                return None;
            }
            if (sched_opt.as_ref().unwrap().get_cpu() == cpu as u32 ||
                sched_opt.as_ref().unwrap().get_cpu() == u32::MAX) {
                let num_waiting = set.len();
                //if let Some(curr_val) = curr_reintro {
                    //if cpu == 0 {
                    //println!("would reintro {:?}", curr_val);
                    //}
                //}
                //println!("removed {} from tree", pid);

                //println!("tree would cpu {} pick {}", cpu, pid);
                //return None;
                //Some(pid)
                // None shouldn't happen anymore
                //getnstimeofday64_rs(&mut step2);
                //let period: u64 = if num_waiting > 7 {
                //if num_waiting > 1 {
                if num_waiting >= 1 {
                //if num_waiting > 2 {
                    let period: u64 = if num_waiting > 8 {
                        //(num_waiting as u64 + 1) * 750000
                        (num_waiting as u64) * 750000
                    } else {
                        6000000
                    };
                    let total_weight;
                    let weight;
                    let inv_weight;
                    //getnstimeofday64_rs(&mut step21);
                    {
                        let state = self.state.as_ref().unwrap().read();
                        //getnstimeofday64_rs(&mut step22);
                        let proc_state = state.get(&pid).unwrap();
                        let prio = proc_state.prio;
                        //let prio = 120;
                        weight = sched_core::SCHED_PRIO_TO_WEIGHT[(prio - 100) as usize] as u64;
                        //let cpu_state = self.cpu_state.as_ref().unwrap().read();
                        //if cpu_state.get(&(cpu as u32)).is_none() {
                        //    println!("cpu is {}", cpu);
                        //}
                        total_weight = cpu_state.weight;
                        inv_weight = cpu_state.inv_weight;
                        //total_weight = cpu_state.get(&(cpu as u32)).unwrap().weight;
                        //inv_weight = cpu_state.get(&(cpu as u32)).unwrap().inv_weight;
                        //total_weight += weight;
                        //for (_, other_pid) in set.iter() {
                        //    let other_state = state.get(&other_pid).unwrap();
                        //    let other_prio = other_state.prio;
                        //    let other_weight = sched_core::SCHED_PRIO_TO_WEIGHT[(other_prio - 100) as usize] as u64;
                        //    total_weight += other_weight;
                        //}
                        //getnstimeofday64_rs(&mut step23);
                    }
                    //getnstimeofday64_rs(&mut step24);
                    //getnstimeofday64_rs(&mut step3);
                    let weight_shift = weight >> 10;
                    //let total_weight_shift = total_weight >> 10;
                    //let total_weight_shift = weight >> 10;
                    //let inv_weight = 0xffffffff / total_weight_shift;
                    let factor = weight_shift * inv_weight;
                    let slice_calc = period * factor;
                    let slice = slice_calc >> 32;
                    //println!("w {} total w {}, w shift {}, tw shift {}, inv {}, factor {}, slice_calc {}, slice {}",
                     //        weight, total_weight, weight_shift, total_weight_shift, inv_weight, factor, slice_calc, slice);
                    //getnstimeofday64_rs(&mut step4);
                    //if cpu_state.should_report % 10000 == 0 {
                    //println!("waiting {} w {} total w {}, w shift {}, inv {}, factor {}, slice_calc {}, slice {}",
                    //        num_waiting, weight, total_weight, weight_shift, inv_weight, factor, slice_calc, slice);
                    ////    //println!("num_waiting {} slice {}", num_waiting, slice);
                    //}
                    hrtick::hrtick_start(cpu, slice);
                }
                //let should_report = REPORT.load(core::sync::atomic::Ordering::Relaxed);
                //if (should_report % 10000 == 0) {
                //    let diff1 = diff_ns(&step1, &start);
                //    let diff11 = diff_ns(&step11, &step1);
                //    let diff12 = diff_ns(&step12, &step11);
                //    let diff2 = diff_ns(&step2, &step12);
                //    let diff21 = diff_ns(&step21, &step2);
                //    let diff22 = diff_ns(&step22, &step21);
                //    let diff23 = diff_ns(&step23, &step22);
                //    let diff24 = diff_ns(&step24, &step23);
                //    let diff3 = diff_ns(&step3, &step24);
                //    let diff4 = diff_ns(&step4, &step3);
                //    let diff5 = diff_ns(&end, &step4);
                //    println!("pnt inner took {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}", diff1, diff11, diff12, diff2, diff21, diff22, diff23, diff24, diff3, diff4, diff5, num_waiting);
                //}
                //REPORT.store(should_report + 1, core::sync::atomic::Ordering::SeqCst);
                cpu_state.curr = Some(sched_opt.as_ref().unwrap().get_pid());
                //cpu_state.exec_start = unsafe {
                //    bento::bindings::cpu_clock_task(cpu)
                //};
                //getnstimeofday64_rs(&mut end);
                //let diff = diff_ns(&end, &start);
                if cpu_state.should_report % 10000 == 0 {
                    //if num_waiting >= 1 {
                     //   println!("cpu {} waiting {}", cpu, num_waiting);
                    //}
                ////if diff > 1000 {
                ////println!("pnt inner took {}", diff);
                //    let diff1 = diff_ns(&step1, &start);
                //    let diff11 = diff_ns(&step11, &step1);
                //    let diff12 = diff_ns(&step12, &step11);
                //    let diff2 = diff_ns(&step2, &step12);
                //    //let diff21 = diff_ns(&step21, &step2);
                //    //let diff22 = diff_ns(&step22, &step21);
                //    //let diff23 = diff_ns(&step23, &step22);
                //    //let diff24 = diff_ns(&step24, &step23);
                //    //let diff3 = diff_ns(&step3, &step24);
                ////    let diff4 = diff_ns(&step4, &step3);
                //    let diff5 = diff_ns(&end, &step2);
                //    println!("pnt inner took {}, {}, {}, {}, {}", diff1, diff11, diff12, diff2, diff5);
                ////}
                ////if cpu == 0 {
                //println!("pick next task will return {} for cpu {} runtime {}", sched_opt.as_ref().unwrap().get_pid(), cpu, vruntime);
                ////}
                }
                cpu_state.should_report += 1;
                cpu_state.should_report += 1;
                cpu_state.load = cpu_state.load << 1;
                cpu_state.capacity = cpu_state.capacity << 1;
                if num_waiting >= 1 {
                    cpu_state.load = cpu_state.load | 1;
                    //println!("new load is {:b}", cpu_state.load);
                }
                //return None;
                return Some(sched_opt.unwrap());
            } else {
                //println!("can't schedule on other cpu\n");
                //q.push_front(pid);
                //println!("pid has the wrong sched {} {} {}", pid, cpu, sched_opt.unwrap().get_cpu());
                //set.insert((vruntime, pid));
                cpu_state.curr = None;
                return None;
            }
        } else {
            //let set1_len = set.len();
            //let mut cpu_state = all_cpu_state.get(&(u32::MAX)).unwrap().write();
            //let set = &mut cpu_state.set;
            //let set2_len = set.len();
            //println!("no task found {} {}", set1_len, set2_len);
            //let mut set = sets_list.get(&(u32::MAX)).unwrap().write();
            //if let Some((vruntime, pid)) = set.pop_first() {
            //    let map = self.map.as_ref().unwrap().read();
            //    if map.get(&pid).is_none() {
            //        println!("Terrible problem, pid has no cpu");
            //        set.insert((vruntime, pid));
            //        return None;
            //    }
            //    if (map.get(&pid).unwrap().get_cpu() == cpu as u32 ||
            //        map.get(&pid).unwrap().get_cpu() == u32::MAX) {
            //        //println!("removed {} from tree", pid);

            ////        println!("tree would cpu {} pick {}", cpu, pid);
            //        //return None;
            //        //Some(pid)
            //        // None shouldn't happen anymore
            //        return Some(*map.get(&pid).unwrap());
            //    } else {
            //        //println!("can't schedule on other cpu\n");
            //        //q.push_front(pid);
            //        set.insert((vruntime, pid));
            //        return None;
            //    }
            //    return None;
            //}
        }
            //if cpu == 0 {
            //println!("cpu {} is empty", cpu);
            //}
        //cpu_state.load = 0;
        //cpu_state.load = 0;
        cpu_state.load = cpu_state.load << 1;
        //cpu_state.load = 0;
        cpu_state.capacity = cpu_state.capacity << 1;
        cpu_state.capacity = cpu_state.capacity | 1;
        cpu_state.curr = None;
        return None;
        }
    }

    fn pnt_err(&self, cpu: i32, pid: u64, err: i32, _sched: Option<Schedulable>, _guard: RQLockGuard) {
        println!("pnt err insert {} {} {}", pid, cpu, err);
        if err == 2 {
            let sched = _sched.unwrap();
           // println!("pnt err insert {} {}", sched.get_pid(), sched.get_cpu());
            let mut map = self.map.as_ref().unwrap().write();
            map.insert(sched.get_pid(), sched);
            //let cpu_state = self.cpu_state.as_ref().unwrap().read();
            //if cpu_state.get(&(sched.get_cpu() as u32)).is_none() {
            //    println!("pnt err cpu is {}", sched.get_cpu());
            //}
        }
        //if err == 1 {
        let prio;
        {
            let mut state = self.state.as_ref().unwrap().write();
            let procstate = state.get_mut(&pid).unwrap();
            prio = procstate.prio;
        }
        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&(cpu as u32)).unwrap().write();
        cpu_state.curr = None;
        if err == 1 {
            //println!("err pid {} cpu {}", pid, cpu);
            self.remove_weight(&mut cpu_state, prio);
        }
        //}
        // If err == 1, the process is actually unschedulable. Dead without telling us for some
        // reason
    }

    fn balance_err(&self, _cpu: i32, pid: u64, _err: i32, _sched: Option<Schedulable>, _guard: RQLockGuard) {
        //let mut map = self.map.as_ref().unwrap().write();
        //let old_sched = map.get(&pid).unwrap();
        //let old_cpu = old_sched.get_cpu();
        let mut balancing = self.balancing.as_ref().unwrap().write();
        balancing.remove(&pid);
    }

    fn select_task_rq(&self, pid: u64, waker_cpu: i32, prev_cpu: i32) -> i32 {
        //{
        //    let mut map = self.map.as_ref().unwrap().write();
        //    if let Some(sched) = map.get(&pid) {
        //        return sched.get_cpu() as i32;
        //    }
        //}
        //let cpu_state = self.cpu_state.as_ref().unwrap().read();
        //let mut cpu = pid % 6;
        ////let mut smallest_size = 1000;
        //let mut largest_capacity = 0;
        //if let Some(lock) = cpu_state.get(&(cpu as u32)) {
        //    let state = lock.read();
        //    let set = &state.set;
        //    if state.capacity.count_ones() == 8 || set.is_empty() {
        //        return cpu as i32;
        //    }
        //    largest_capacity = state.capacity.count_ones();
        //    //let set = &state.set;
        //    //if set.is_empty() {
        //    //    return cpu as i32;
        //    //}
        //    //cpu = 0;
        //    //smallest_size = set.len();
        //}
        //for i in 0..6 {
        //    if i == pid % 6 {
        //        continue;
        //    }
        //    if let Some(lock) = cpu_state.get(&(i as u32)) {
        //        let state = lock.read();
        //        let set = &state.set;
        //        if state.capacity.count_ones() == 8 || set.is_empty() {
        //            return i as i32;
        //        }
        //    //smallest_size = state.capacity.count_ones();
        //        //if set.is_empty() {
        //        //    return i as i32;
        //        //}
        //        //if set.len() < smallest_size {
        //        if state.capacity.count_ones() > largest_capacity {
        //            cpu = i;
        //            largest_capacity = state.capacity.count_ones();
        //            //smallest_size = set.len();
        //        }
        //    }
        //}
        //return cpu as i32;

        let state = self.state.as_ref().unwrap().read();
        //let proc_state = state.get_mut(&pid).unwrap();
        //let mut map = self.map.as_ref().unwrap().write();
        match state.get(&pid) {
            None => {
                //map.insert(pid, pid as u32 % 2);
                //println!("would pick {}, waker {}, prev {}", pid % 6, waker_cpu, prev_cpu);
                pid as i32 % 6
                //4
                //prev_cpu
            },
            //None => 5,
            Some(proc_state) => {
                //println!("moving {} {} to {}", pid, prev_cpu, waker_cpu);
                proc_state.cpu as i32
                //sched.get_cpu() as i32
            },
        }
    }

    fn selected_task_rq(&self, sched: Schedulable) {
        //println!("selected {} {}", sched.get_pid(), sched.get_cpu());
        //let mut state = self.state.as_ref().unwrap().write();
        //let mut map = self.map.as_ref().unwrap().write();
        //if map.get(&sched.get_pid()).is_none() {
        //    map.insert(sched.get_pid(), sched);
        //}
        //map.insert(sched.get_pid(), sched);
        //let cpu_state = self.cpu_state.as_ref().unwrap().read();
        //if cpu_state.get(&(sched.get_cpu() as u32)).is_none() {
        //    println!("selected cpu is {}", sched.get_cpu());
        //}
    }

    //fn migrate_task_rq(&self, pid: u64, new_cpu: i32) {
    fn migrate_task_rq(&self, pid: u64, sched: Schedulable, _guard: RQLockGuard) -> Schedulable {
        //println!("hitting migrate_task_rq pid {} to cpu {}", pid, sched.get_cpu());
        let old_cpu;
        let cpu = sched.get_cpu();
        let old_sched;
        {
            let mut map = self.map.as_ref().unwrap().write();
            old_sched = map.remove(&pid).unwrap();
            old_cpu = old_sched.get_cpu();
            map.insert(pid, sched);
        }
        let vruntime;
        let prio;
        let runtime;
        {
            let mut state = self.state.as_ref().unwrap().write();
            let procstate = state.get_mut(&pid).unwrap();
            vruntime = procstate.vruntime;
            runtime = procstate.last_runtime;
            prio = procstate.prio;
            procstate.cpu = cpu;
            //procstate.vruntime = calculate_vruntime(runtime, procstate.prio);
            //vruntime = procstate.vruntime;
        }
        //println!("migrate old cpu {}", old_cpu);
        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        if old_cpu != u32::MAX {
            let mut cpu_state = all_cpu_state.get(&old_cpu).unwrap().write();
            //let mut real_set = cpu_state.set.get(&cpu).unwrap().write();
            let mut real_set = &mut cpu_state.set;
            let was_in = real_set.remove(&(vruntime, pid));
            //println!("did remove old sched {}", was_in);
            //println!("removed from set {:?}", real_set);
            //println!("migrate remove pid {} cpu {} set is {:?}", pid, old_cpu, real_set);
            self.remove_weight(&mut cpu_state, prio);
            cpu_state.free_time = 0;
            //cpu_state.load = 0;
            cpu_state.load = cpu_state.load << 1;
            if Some(pid) == cpu_state.curr {
                cpu_state.curr = None;
            }
        }
        //let cpu_state = self.cpu_state.as_ref().unwrap().read();
        //if cpu_state.get(&(sched.get_cpu() as u32)).is_none() {
        //    println!("migrate cpu is {}", sched.get_cpu());
        //}
        let updated_vruntime = vruntime;
        {
            let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
            let mut real_set = &mut cpu_state.set;
            let raw_vruntime = calculate_vruntime(runtime, prio);
            //updated_vruntime = if vruntime > raw_vruntime {
            //    if let Some((min_vruntime, _)) = real_set.first() {
            //        core::cmp::max(raw_vruntime, *min_vruntime)
            //    } else {
            //        raw_vruntime
            //    }
            //} else {
            //    vruntime
            //};
            real_set.insert((updated_vruntime, pid));
            //println!("added to set {:?}", real_set);
            //println!("migrate add pid {} cpu {} set is {:?}", pid, sched.get_cpu(), real_set);
            self.add_weight(&mut cpu_state, prio);
            cpu_state.free_time = 0;
            //cpu_state.capacity = 0;
            cpu_state.capacity = cpu_state.capacity << 1;
        }
        if updated_vruntime != vruntime {
            let mut state = self.state.as_ref().unwrap().write();
            let procstate = state.get_mut(&pid).unwrap();
            procstate.vruntime = updated_vruntime;
        }
        let mut balancing = self.balancing.as_ref().unwrap().write();
        balancing.remove(&pid);
        return old_sched;
        //let mut balancing_cpus = self.balancing_cpus.as_ref().unwrap().write();
        //balancing_cpus.remove(&old_cpu);
    }

    fn task_affinity_changed(&self, pid: u64, cpumask: u64) {
        //println!("changing affinity for {} {}", pid, cpumask);
        let mut locked = self.locked.as_ref().unwrap().write();
        locked.insert(pid);
    }

    fn balance(&self, cpu: i32, _guard: RQLockGuard) -> Option<u64> {
        //println!("balance for cpu {}", cpu);
        //return None;
        let cpu_state = self.cpu_state.as_ref().unwrap().read();
        if cpu_state.get(&(cpu as u32)).is_none() {
            //println!("balance no set");
        } else if let Some(lock) = cpu_state.get(&(cpu as u32)) {
            let state = lock.read();
        //    if state.free_time == 0 {
        //        return None;
        //    } else {
        //        //let mut min_cpu = None;
        //        let mut min_free_time = state.free_time;
        //        let mut pid = None;
        //        for i in 0..6 {
        //            if i == cpu {
        //                continue;
        //            }
        //            if let Some(lock2) = cpu_state.get(&(i as u32)) {
        //                let state2 = lock2.read();
        //                if (state2.free_time + 10) < min_free_time {
        //                    //min_cpu = Some(i);
        //                    let mut balancing = self.balancing.as_ref().unwrap().write();
        //                    let set2 = &state2.set;
        //                    let opt_pid = set2.last().and_then(|(_, p)| Some(*p));
        //                    if let Some(choice_pid) = opt_pid {
        //                        if !balancing.contains(&choice_pid) {
        //                            min_free_time = state2.free_time;
        //                            pid = opt_pid;
        //                            balancing.insert(choice_pid);
        //                        }
        //                    }
        //                }
        //            }
        //        }
        //        if pid.is_some() {
        //        println!("core {} wanting to move pid {:?}", cpu, pid);
        //        }
        //        return pid;
        //    }
        //}
        //                let set2 = &state2.set;
        //                if set2.len() > 1 {
        //                    for (_, pid) in set2 {
        //                        let mut balancing = self.balancing.as_ref().unwrap().write();
        //                        if Some(*pid) != state2.curr && !balancing.contains(pid) {
        //                            //println!("balance wants to move to empty set {}", *pid);
        //                            balancing.insert(*pid);
        //                            //return None;
        //                            return Some(*pid);
        //                        }
        //                    }
        //                    //if set2.last() != state2.curr {
        //                    //}
        //                    //println!("balance wants to move to empty set");
        //                    //break;
        //                }
        //            }
            //}
            let self_free = state.free_time;
            let idle = state.curr.is_none();
            let set = &state.set;
            //println!("cpu {} set len {} idle {}", cpu, set.len(), idle);
            //if set.is_empty() && idle && state.load == 0 && state.capacity.count_ones() >= 5 {
            //if state.load == 0 && state.capacity.count_ones() >= 6 {
            //if !(set.is_empty() && idle) && !(state.capacity.count_ones() >=6) {
                //println!("full cpu {} len {} idle {} ones {}", cpu, set.len(), idle, state.capacity.count_ones());
            //}
            if state.capacity.count_ones() >= 5 {
            //if (set.is_empty() && idle) || state.capacity.count_ones() >= 5 {
                {
                    let mut balancing_cpus = self.balancing_cpus.as_ref().unwrap().write();
                    //if balancing_cpus.contains_key(&(cpu as u32)) {
                        //let num_runs = balancing_cpus.get_mut(&(cpu as u32)).unwrap();
                        //if *num_runs >= 3 {
                        //    balancing_cpus.remove(&(cpu as u32));
                        //    return None;
                        //} else {
                        //    *num_runs += 1;
                        //}
                    //} else {
                        balancing_cpus.insert(cpu as u32, 1);
                    //}
                }
            //if set.is_empty() && idle {
                //println!("balance found blank cpu {} load {} capacity {}", cpu, state.load, state.capacity.count_ones());
                //return None;
                let mut max_cpu = None;
                let mut max_tasks = 0;
                //let mut max_load = 0;
                let mut max_load = state.load.count_ones();
                let mut balance_pid = None;
                //let mut other_free = 0;
                for i in 0..6 {
                    if i == cpu {
                        continue;
                    }
                    {
                        let balancing_cpus = self.balancing_cpus.as_ref().unwrap().read();
                        if balancing_cpus.contains_key(&(i as u32)) {
                            continue;
                        }
                    }
                    if let Some(lock2) = cpu_state.get(&(i as u32)) {
                        let state2 = lock2.read();
                        let idle2 = state2.curr.is_none();
                        let set2 = &state2.set;
                //        if set2.len() > max_tasks && !(idle2 && set2.len() == 1) {
                        if state2.load.count_ones() > max_load && !(idle2 && set2.len() == 1) && state2.capacity.count_ones() < 4 {
                        //if state2.load.count_ones() > max_load && !(set2.len() <= 1) && state2.capacity.count_ones() < 6 {
                            for (_, pid) in set2 {
                                let mut balancing = self.balancing.as_ref().unwrap().read();
                                let mut locked = self.locked.as_ref().unwrap().read();
                                if Some(*pid) != state2.curr && !balancing.contains(pid) && !locked.contains(pid) {
                                    max_cpu = Some(i);
                                    max_tasks = set2.len();
                                    max_load = state2.load.count_ones();
                                    balance_pid = Some(*pid);
                        //            balancing.insert(*pid);
                         //           return balance_pid;
                                    //other_free = state2.free_time;
                                    break;
                                }
                            }
                        //} else {
                         //   println!("wants to balance cpu {} to {} but failed, max load {} load {}, idle2 {} len {}, cpu 2 capacity {}",
                          //           i, cpu, max_load, state2.load.count_ones(), idle2, set2.len(), state2.capacity.count_ones());
                        }
                        //if state2.load != 0 {
                        //    println!("core {} set len {}, set load {:b}", i, set2.len(), state2.load);
                        //}
                        //if set2.len() > max_tasks && !(idle2 && set2.len() == 1) {
                        //    println!("checking set for core {} idle {} {:?}", i, idle2, set2);
                        //    for (_, pid) in set2 {
                        //        let mut balancing = self.balancing.as_ref().unwrap().write();
                        //        let mut locked = self.locked.as_ref().unwrap().read();
                        //        if Some(*pid) != state2.curr && !balancing.contains(pid) && !locked.contains(pid) {
                        //            max_cpu = Some(i);
                        //            max_tasks = set2.len();
                        //            balance_pid = Some(*pid);
                        //            balancing.insert(*pid);
                        //            other_free = state2.free_time;
                        //            break;
                        //        } else {
                        //            //if Some(*pid) == state2.curr {
                        //            //    //println!("is curr");
                        //            //}
                        //            //if balancing.contains(pid) {
                        //            //    println!("already balancing pid");
                        //            //}
                        //            //if locked.contains(pid) {
                        //            //    println!("cannot move pid");
                        //            //}
                        //        }
                        //    }
                        ////let opt_pid = set2.last().and_then(|(_, p)| Some(*p));
                        //    //max_cpu = Some(i);
                        //    //balance_pid = set2.first().
                        //}
                        //if set2.len() > 1 {
                            
        //                    for (_, pid) in set2 {
        //                        let mut balancing = self.balancing.as_ref().unwrap().write();
        //                        if Some(*pid) != state2.curr && !balancing.contains(pid) {
        //                            //println!("balance wants to move to empty set {}", *pid);
        //                            balancing.insert(*pid);
        //                            //return None;
        //                            return Some(*pid);
        //                        }
        //                    }
        //                    //if set2.last() != state2.curr {
        //                    //}
        //                    //println!("balance wants to move to empty set");
        //                    //break;
        //                }
                    }
                }
                if let Some(bpid) = balance_pid && let Some(other_cpu) = max_cpu {
                    //println!("balacing {:?} from {:?} to {}", balance_pid, max_cpu, cpu);
                    let mut balancing = self.balancing.as_ref().unwrap().write();
                    if !balancing.insert(bpid) {
                        // race, it's already in
                        balance_pid = None;
                    }
                    //let mut balancing_cpus = self.balancing_cpus.as_ref().unwrap().write();
                    //balancing_cpus.insert(other_cpu as u32);
                }
                if balance_pid.is_none() {
                    let mut balancing_cpus = self.balancing_cpus.as_ref().unwrap().write();
                    balancing_cpus.remove(&(cpu as u32));
                }
                return balance_pid;
                //return None;
            }
        }
        //if cpu != 1 {
        //    return None;
        //}
        //let q = self.q.as_ref().unwrap().read();
        //let mut next = &0;
        //if let Some(opt) = q.front() {
        //    next = opt;
        //} else {
        //    return None;
        //}
        //let mut map = self.map.as_ref().unwrap().write();
        //if let Some(0) = map.get(next) {
        //    println!("shifting proc {} to cpu 1", *next);
        //    map.insert(*next, 1);
        //    return Some(*next);
        //}
        let mut balancing_cpus = self.balancing_cpus.as_ref().unwrap().write();
        balancing_cpus.remove(&(cpu as u32));
        return None;
    }

    fn task_yield(
        &self, pid: u64, runtime: u64,
        _cpu_seqnum: u64, cpu: i32, _from_switchto: i8, sched: Schedulable, _guard: RQLockGuard
    ) {
        //if cpu == 0 {
        //println!("yielding {} {}", pid, cpu);
        //}
        //let mut new_sched = false;
        //{
        //    let mut state = self.state.as_ref().unwrap().write();
        //    let procstate = state.get_mut(&pid).unwrap();
        //    //old_vruntime = procstate.vruntime;
        //    //prio = procstate.prio;
        //    //procstate.vruntime += runtime;
        //    procstate.vruntime = calculate_vruntime(runtime, procstate.prio);
        //}
        //{
        //    let mut map = self.map.as_ref().unwrap().write();
        //    if let Some(old_sched) = map.get(&pid) {
        //        if old_sched.get_cpu() != sched.get_cpu() {
        //            new_sched = true;
        //        }
        //    }
        //    map.insert(pid, sched);
        //}
        let cpu = sched.get_cpu();
        {
            let mut map = self.map.as_ref().unwrap().write();
            if map.get(&pid).is_none() {
                map.insert(pid, sched);
            }
        }
        let mut state = self.state.as_ref().unwrap().write();
        let proc_state = state.get_mut(&pid).unwrap();
        let old_vruntime = proc_state.vruntime;
        let delta_runtime = runtime - proc_state.last_runtime;
        proc_state.vruntime += calculate_vruntime(delta_runtime, proc_state.prio);
            //let other_vruntime = calculate_vruntime(runtime, proc_state.prio);
            //if other_vruntime != proc_state.vruntime {
            //    println!("yield old vruntime {} new {}", other_vruntime, proc_state.vruntime);
            //}
        //proc_state.vruntime = calculate_vruntime(runtime, proc_state.prio);
        proc_state.last_runtime = runtime;
        proc_state.cpu = cpu;

        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
        let mut real_set = &mut cpu_state.set;
        let remove_val = (old_vruntime, pid);
        real_set.remove(&remove_val);
        real_set.insert((proc_state.vruntime, pid));
        if cpu_state.curr == Some(pid) {
            cpu_state.curr = None;
        }
        //if new_sched {
        //    self.add_weight(&mut cpu_state, proc_state.prio);
        //}
    }

    fn reregister_prepare(&mut self) -> Option<UpgradeData> {
        let mut data = UpgradeData {
            map: None,
            state: None,
            cpu_state: None,
            user_q: None,
            rev_q: None,
            balancing: None,
            balancing_cpus: None,
            locked: None
        };
        mem::swap(&mut data.map, &mut self.map);
        mem::swap(&mut data.state, &mut self.state);
        mem::swap(&mut data.cpu_state, &mut self.cpu_state);
        mem::swap(&mut data.user_q, &mut self.user_q);
        mem::swap(&mut data.rev_q, &mut self.rev_q);
        mem::swap(&mut data.balancing, &mut self.balancing);
        mem::swap(&mut data.balancing_cpus, &mut self.balancing_cpus);
        mem::swap(&mut data.locked, &mut self.locked);
        return Some(data);
    }

    fn reregister_init(&mut self, data_opt: Option<UpgradeData>) {
        if let Some(mut data) = data_opt {
            mem::swap(&mut self.map, &mut data.map);
            mem::swap(&mut self.state, &mut data.state);
            mem::swap(&mut self.cpu_state, &mut data.cpu_state);
            mem::swap(&mut self.user_q, &mut data.user_q);
            mem::swap(&mut self.rev_q, &mut data.rev_q);
            mem::swap(&mut self.balancing, &mut data.balancing);
            mem::swap(&mut self.balancing_cpus, &mut data.balancing_cpus);
            mem::swap(&mut self.locked, &mut data.locked);
        }
    }

    fn register_queue(&self, q: RingBuffer<UserMessage>) -> i32 {
        //println!("q ptr {:?}", q.inner);
        //println!("q ptr {:?}", (*q.inner).offset);
        //println!("q capacity {}", (*q.inner).capacity);
        //println!("q readptr {}", (*q.inner).readptr);
        //println!("q writeptr {}", (*q.inner).writeptr);
        let mut user_q = self.user_q.as_ref().unwrap().write();
        let next = user_q.keys().max().map_or(0, |max| max + 1);
        user_q.insert(next as i32, q);
        return next as i32;

        //self.user_q = Some(RwLock::new(q));
    }

    fn register_reverse_queue(&self, q: RingBuffer<UserMessage>) -> i32 {
        //println!("q ptr {:?}", q.inner);
        //println!("q ptr {:?}", (*q.inner).offset);
        //println!("q capacity {}", (*q.inner).capacity);
        //println!("q readptr {}", (*q.inner).readptr);
        //println!("q writeptr {}", (*q.inner).writeptr);
        let mut rev_q = self.rev_q.as_ref().unwrap().write();
        let next = rev_q.keys().max().map_or(0, |max| max + 1);
        rev_q.insert(next as i32, q);
        return next as i32;

        //self.user_q = Some(RwLock::new(q));
    }

    fn enter_queue(&self, id: i32, entries: u32) {
        let mut user_q_list = self.user_q.as_ref().unwrap().write();
        let user_q = user_q_list.get_mut(&id).unwrap();
        for i in 0..entries {
            let msg = user_q.dequeue();
        println!("msg val {:?}", msg);
        }
    }

    fn unregister_queue(&self, id: i32) -> RingBuffer<UserMessage> {
        let mut user_q = self.user_q.as_ref().unwrap().write();
        let q = user_q.remove(&id);
        println!("freeing queue");
        // We must have a q or this won't be called
        q.unwrap()
    }

    fn unregister_rev_queue(&self, id: i32) -> RingBuffer<UserMessage> {
        let mut rev_q = self.rev_q.as_ref().unwrap().write();
        let q = rev_q.remove(&id);
        println!("freeing queue");
        // We must have a q or this won't be called
        q.unwrap()
    }

    fn task_tick(&self, cpu: i32, queued: bool, guard: RQLockGuard) {
        //self.update_curr(cpu);
        //let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        //let mut cpu_state = all_cpu_state.get(&(cpu as u32)).unwrap().write();
        //if let Some(curr) = cpu_state.curr {
        //    let now = unsafe {
        //        bento::bindings::cpu_clock_task(cpu)
        //    };
        //    let delta_runtime = now - cpu_state.exec_start;
        //    let mut state = self.state.as_ref().unwrap().write();
        //    let proc_state = state.get_mut(&curr).unwrap();
        //    let old_vruntime = proc_state.vruntime;
        //    proc_state.vruntime = old_vruntime + calculate_vruntime(delta_runtime, proc_state.prio);
        //    println!("tick updating {} on {} to {}", curr, cpu, proc_state.vruntime);
        //    let remove_val = (old_vruntime, curr);
        //    let mut real_set = &mut cpu_state.set;
        //    real_set.remove(&remove_val);
        //    real_set.insert((proc_state.vruntime, curr));
        //    cpu_state.exec_start = unsafe {
        //        bento::bindings::cpu_clock_task(cpu)
        //    };
        //} else {
        //    println!("ticking but curr empty for cpu {}", cpu);
        //}
        if queued {
            //println!("ticking {}", cpu);
            resched_cpu(cpu, &guard);
            //unsafe {
            //    bento::bindings::resched_cpu_no_lock(cpu);
            //}
        }
    }
}

//bento::kernel_module!(
//    BentoGhostModule,
//    author: b"Bento Contributors",
//    description: b"kernel module to replace the scheduler",
//    license: b"GPL"
//);
