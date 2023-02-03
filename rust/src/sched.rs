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

use self::ringbuffer::RingBuffer;
use bento::kernel::time::Timespec64;
use bento::kernel::time::getnstimeofday64_rs;
use bento::kernel::time::diff_ns;
static REPORT: core::sync::atomic::AtomicI64 = core::sync::atomic::AtomicI64::new(0);

#[repr(C)]
pub struct BentoSched {
//    pub sets_list: Option<RwLock<BTreeMap<u32, RwLock<BTreeSet<(u64,u64)>>>>>,
    pub map: Option<RwLock<BTreeMap<u64, Schedulable>>>,
    pub state: Option<RwLock<BTreeMap<u64, ProcessState>>>,
    pub cpu_state: Option<RwLock<BTreeMap<u32, RwLock<CpuState>>>>,
    pub user_q: Option<RwLock<Option<RingBuffer<UserMessage>>>>,
    pub rev_q: Option<RwLock<Option<RingBuffer<UserMessage>>>>,
}

pub struct ProcessState {
    prio: i32,
    vruntime: u64
}

#[derive(Default)]
pub struct CpuState {
    pub weight: u64,
    pub inv_weight: u64,
    pub set: BTreeSet<(u64, u64)>,
}

pub struct UpgradeData {
    pub cpu_state: Option<RwLock<BTreeMap<u32, RwLock<CpuState>>>>,
    //pub sets_list: Option<RwLock<BTreeMap<u32, RwLock<BTreeSet<(u64,u64)>>>>>,
    map: Option<RwLock<BTreeMap<u64, Schedulable>>>
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct UserMessage {
    val: i32
}

fn calculate_vruntime(runtime: u64, prio: i32) -> u64 {
    let inv_weight = sched_core::SCHED_PRIO_TO_WMULT[(prio - 100) as usize] as u64;
    let factor = 1024 * inv_weight;
    let runtime_calc = runtime as u64 * factor;
    runtime_calc >> 32
}

impl BentoSched {
    fn add_weight(&self, cpu_state: &mut CpuState, prio: i32) {
        //let mut cpu_state = self.cpu_state.as_ref().unwrap().write();
        //let curr_weight_opt = cpu_state.get_mut(&cpu);
        let weight = sched_core::SCHED_PRIO_TO_WEIGHT[(prio - 100) as usize] as u64;
        //if curr_weight_opt.is_some() {
            //println!("got weight {:?}", curr_weight_opt);
  //          let curr_weight = curr_weight_opt.unwrap();
            cpu_state.weight += weight;
            let weight_shift = cpu_state.weight >> 10;
            let inv_weight = 0xffffffff / weight_shift;
            cpu_state.inv_weight = inv_weight;
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
}

impl BentoScheduler<'_, '_, UpgradeData, UpgradeData, UserMessage, UserMessage> for BentoSched {
    fn get_policy(&self) -> i32 {
        10
    }

    //fn init(&mut self) {
   // }

    fn task_new(&self, pid: u64, runtime: u64, runnable: u16, prio: i32, sched: Schedulable) {
        //println!("task new pid {} prio {} runtime {}", pid, prio, runtime);
        let vruntime = calculate_vruntime(runtime, prio);
        {
            let mut state = self.state.as_ref().unwrap().write();
            let procstate = ProcessState {
                prio: prio,
                vruntime: vruntime,
            };
            state.insert(pid, procstate);
        }
        if runnable > 0 {
            let mut map = self.map.as_ref().unwrap().write();
            if map.get(&pid).is_none() {
                map.insert(pid, sched);
            }
            let cpu = map.get(&pid).unwrap().get_cpu();
            //println!("cpu is {}", cpu);
            //let sets_list = self.sets_list.as_ref().unwrap().read();
            //let cpu_state
            let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
            let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
            //let mut real_set = cpu_state.set.get(&cpu).unwrap().write();
            let mut real_set = &mut cpu_state.set;
            real_set.insert((vruntime, pid));
            self.add_weight(&mut cpu_state, prio);
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

    fn task_prio_changed(&self, pid: u64, _prio: i32) {
        //println!("task prio changed {}", pid);
    }

    fn task_wakeup(&self, pid: u64, _agent_data: u64, deferrable: bool,
                   _last_run_cpu: i32, _wake_up_cpu: i32, _waker_cpu: i32,
                   sched: Schedulable) {
        let mut new_sched = false;
        {
            let mut map = self.map.as_ref().unwrap().write();
            if let Some(old_sched) = map.get(&pid) {
                if old_sched.get_cpu() != sched.get_cpu() {
                    new_sched = true;
                }
            }
            map.insert(pid, sched);
        }
        let mut state = self.state.as_ref().unwrap().read();
        let proc_state = state.get(&pid).unwrap();
        let cpu = sched.get_cpu();
        //let sets_list = self.sets_list.as_ref().unwrap().read();
        //let mut real_set = sets_list.get(&cpu).unwrap().write();
        //real_set.insert((proc_state.vruntime, pid));

        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
        //let mut real_set = cpu_state.set.get(&cpu).unwrap().write();
        let mut real_set = &mut cpu_state.set;
        real_set.insert((proc_state.vruntime, pid));
        //self.add_weight(&mut cpu_state, prio);
        //let cpu_state = self.cpu_state.as_ref().unwrap().read();
        //if cpu_state.get(&(cpu as u32)).is_none() {
        //    println!("wakeup cpu is {}", cpu);
        //}
        if new_sched {
            self.add_weight(&mut cpu_state, proc_state.prio);
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
        }
        //let cpu_state = self.cpu_state.as_ref().unwrap().read();
        //if cpu_state.get(&(cpu as u32)).is_none() {
        //    println!("wakeup cpu is {}", cpu);
        //}
        //println!("added {} to tree", pid);
    }

    fn task_preempt(&self, pid: u64, runtime: u64, _cpu_seqnum: u64, _cpu: i32,
                    _from_switchto: i8, _was_latched: i8, sched: Schedulable) {
        let vruntime;
        let old_vruntime;
        {
            let mut state = self.state.as_ref().unwrap().write();
            let procstate = state.get_mut(&pid).unwrap();
            old_vruntime = procstate.vruntime;
            procstate.vruntime = calculate_vruntime(runtime, procstate.prio);
            vruntime = procstate.vruntime;
        }
        let mut map = self.map.as_ref().unwrap().write();
        map.insert(pid, sched);
        let cpu = sched.get_cpu();
        //let sets_list = self.sets_list.as_ref().unwrap().read();
        //let mut real_set = sets_list.get(&cpu).unwrap().write();
        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
        let mut real_set = &mut cpu_state.set;
        real_set.insert((vruntime, pid));
        ////let cpu_state = self.cpu_state.as_ref().unwrap().read();
        //if cpu_state.get(&(cpu as u32)).is_none() {
        //    println!("preempt cpu is {}", cpu);
        //}
        //println!("added {} to tree", pid);
        //hrtick::hrtick_start(_cpu, 10000);
    }

    fn task_blocked(&self, pid: u64, runtime: u64, _cpu_seqnum: u64,
                    _cpu: i32, _from_switchto: i8, sched: Schedulable) {
        let old_vruntime;
        let prio;
        {
            let mut state = self.state.as_ref().unwrap().write();
            let procstate = state.get_mut(&pid).unwrap();
            old_vruntime = procstate.vruntime;
            prio = procstate.prio;
            //procstate.vruntime += runtime;
            procstate.vruntime = calculate_vruntime(runtime, procstate.prio);
        }
        let remove_val = (old_vruntime, pid);

        let mut map = self.map.as_ref().unwrap().write();
        let cpu = map.get(&pid).unwrap().get_cpu();
        //let sets_list = self.sets_list.as_ref().unwrap().read();
        //let mut real_set = sets_list.get(&cpu).unwrap().write();
        //real_set.remove(&remove_val);
        
        {
            let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
            let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
            let mut real_set = &mut cpu_state.set;
            real_set.remove(&remove_val);

        }
        //map.insert(pid, cpu as u32);
        let new_sched = sched.get_cpu() != cpu;
        map.insert(pid, sched);
        if new_sched {
            let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
            let mut cpu_state = all_cpu_state.get(&sched.get_cpu()).unwrap().write();
            self.add_weight(&mut cpu_state, prio);
        }
        //let cpu_state = self.cpu_state.as_ref().unwrap().read();
        //if cpu_state.get(&(sched.get_cpu() as u32)).is_none() {
        //    println!("block cpu is {}", sched.get_cpu());
        //}
    }

    fn task_dead(&self, pid: u64) {
        //println!("deadge {}", pid);
        // TODO: Remove weight from cpu
    }

    fn pick_next_task(&self, cpu: i32) -> Option<Schedulable> {
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
        //let mut step4 = Timespec64::new();
        //getnstimeofday64_rs(&mut start);
        //println!("pick next task called");
        //let mut tree = self.tree.as_ref().unwrap().write();
        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&(cpu as u32)).unwrap().write();
        let set = &mut cpu_state.set;
        //let sets_list = self.sets_list.as_ref().unwrap().read();
        //let mut set = sets_list.get(&(cpu as u32)).unwrap().write();
        //getnstimeofday64_rs(&mut step1);
        //let mut set = self.set.as_ref().unwrap().write();
        //if set.len() > 0 {
        if let Some((vruntime, pid)) = set.pop_first() {
            //let mut set = &mut cpu_state.set;
            //getnstimeofday64_rs(&mut step11);
            let sched_opt = {
                let map = self.map.as_ref().unwrap().read();
                map.get(&pid).and_then(|x| Some(*x))
            };
            //getnstimeofday64_rs(&mut step12);
            if sched_opt.is_none() {
                println!("Terrible problem, pid has no cpu");
                set.insert((vruntime, pid));
                return None;
            }
            if (sched_opt.unwrap().get_cpu() == cpu as u32 ||
                sched_opt.unwrap().get_cpu() == u32::MAX) {
                let num_waiting = set.len();
                //println!("removed {} from tree", pid);

                //println!("tree would cpu {} pick {}", cpu, pid);
                //return None;
                //Some(pid)
                // None shouldn't happen anymore
                //getnstimeofday64_rs(&mut step2);
                let period: u64 = if num_waiting > 7 {
                    (num_waiting as u64 + 1) * 750000
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
               // getnstimeofday64_rs(&mut step4);
                if num_waiting > 0 {
                    hrtick::hrtick_start(cpu, slice);
                }
                //getnstimeofday64_rs(&mut end);
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
                //    //let diff = diff_ns(&end, &start);
                //    println!("pnt inner took {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}", diff1, diff11, diff12, diff2, diff21, diff22, diff23, diff24, diff3, diff4, diff5, num_waiting);
                //    //println!("pnt inner took {}", diff);
                //}
                //REPORT.store(should_report + 1, core::sync::atomic::Ordering::SeqCst);
                return Some(sched_opt.unwrap());
            } else {
                //println!("can't schedule on other cpu\n");
                //q.push_front(pid);
                set.insert((vruntime, pid));
                return None;
            }
        } else {
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
        return None;
        }
    }

    fn pnt_err(&self, sched: Schedulable) {
        let mut map = self.map.as_ref().unwrap().write();
        map.insert(sched.get_pid(), sched);
        let cpu_state = self.cpu_state.as_ref().unwrap().read();
        if cpu_state.get(&(sched.get_cpu() as u32)).is_none() {
            println!("pnt err cpu is {}", sched.get_cpu());
        }
    }

    fn select_task_rq(&self, pid: u64) -> i32 {
        let mut map = self.map.as_ref().unwrap().write();
        //*map.get(&pid).unwrap_or(&1) as i32
        match map.get(&pid) {
            //None => {
            //    //map.insert(pid, pid as u32 % 2);
            //    pid as i32 % 2
            //},
            None => 5,
            Some(sched) => sched.get_cpu() as i32,
        }
    }

    fn selected_task_rq(&self, sched: Schedulable) {
        //let mut state = self.state.as_ref().unwrap().write();
        let mut map = self.map.as_ref().unwrap().write();
        map.insert(sched.get_pid(), sched);
        let cpu_state = self.cpu_state.as_ref().unwrap().read();
        if cpu_state.get(&(sched.get_cpu() as u32)).is_none() {
            println!("selected cpu is {}", sched.get_cpu());
        }
    }

    //fn migrate_task_rq(&self, pid: u64, new_cpu: i32) {
    fn migrate_task_rq(&self, pid: u64, sched: Schedulable) {
        //println!("hitting migrate_task_rq pid {} to cpu {}", pid, new_cpu);
        let mut map = self.map.as_ref().unwrap().write();
        map.insert(pid, sched);
        let cpu_state = self.cpu_state.as_ref().unwrap().read();
        if cpu_state.get(&(sched.get_cpu() as u32)).is_none() {
            println!("migrate cpu is {}", sched.get_cpu());
        }
    }

    fn balance(&self, cpu: i32) -> Option<u64> {
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
        return None;
    }

    fn reregister_prepare(&mut self) -> Option<UpgradeData> {
        let mut data = UpgradeData {
            map: None,
            cpu_state: None
        };
        mem::swap(&mut data.map, &mut self.map);
        mem::swap(&mut data.cpu_state, &mut self.cpu_state);
        return Some(data);
    }

    fn reregister_init(&mut self, data_opt: Option<UpgradeData>) {
        if let Some(mut data) = data_opt {
            mem::swap(&mut self.map, &mut data.map);
            mem::swap(&mut self.cpu_state, &mut data.cpu_state);
        }
    }

    fn register_queue(&self, q: RingBuffer<UserMessage>) {
        //println!("q ptr {:?}", q.inner);
        //println!("q ptr {:?}", (*q.inner).offset);
        //println!("q capacity {}", (*q.inner).capacity);
        //println!("q readptr {}", (*q.inner).readptr);
        //println!("q writeptr {}", (*q.inner).writeptr);
        let mut user_q = self.user_q.as_ref().unwrap().write();
        user_q.replace(q);

        //self.user_q = Some(RwLock::new(q));
    }

    fn register_reverse_queue(&self, q: RingBuffer<UserMessage>) {
        //println!("q ptr {:?}", q.inner);
        //println!("q ptr {:?}", (*q.inner).offset);
        //println!("q capacity {}", (*q.inner).capacity);
        //println!("q readptr {}", (*q.inner).readptr);
        //println!("q writeptr {}", (*q.inner).writeptr);
        let mut rev_q = self.rev_q.as_ref().unwrap().write();
        rev_q.replace(q);

        //self.user_q = Some(RwLock::new(q));
    }

    fn enter_queue(&self, entries: u32) {
        let mut user_q_guard = self.user_q.as_ref().unwrap().write();
        let user_q = user_q_guard.as_mut().unwrap();
        for i in 0..entries {
        //unsafe {
        //println!("user_q head {}", (*user_q.inner).writeptr);
        //println!("user_q tail {}", (*user_q.inner).readptr);
        //}
            let msg = user_q.dequeue();
        //println!("msg val {:?}", msg);
        }
    }

    fn unregister_queue(&self) -> RingBuffer<UserMessage> {
        let mut user_q = self.user_q.as_ref().unwrap().write();
        let q = user_q.take();
        println!("freeing queue");
        // We must have a q or this won't be called
        q.unwrap()
    }

    fn unregister_rev_queue(&self) -> RingBuffer<UserMessage> {
        let mut rev_q = self.rev_q.as_ref().unwrap().write();
        let q = rev_q.take();
        println!("freeing queue");
        // We must have a q or this won't be called
        q.unwrap()
    }

    fn task_tick(&self, cpu: i32, queued: bool) {
        if queued {
            //println!("ticking {}", cpu);
            unsafe {
                bento::bindings::resched_cpu_no_lock(cpu);
            }
        }
    }
}

//bento::kernel_module!(
//    BentoGhostModule,
//    author: b"Bento Contributors",
//    description: b"kernel module to replace the scheduler",
//    license: b"GPL"
//);
