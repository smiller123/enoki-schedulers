#[cfg(not(feature = "replay"))]
use bento::println;
#[cfg(not(feature = "replay"))]
use bento::scheduler_utils;
#[cfg(not(feature = "replay"))]
use bento::spin_rs::RwLock;
#[cfg(not(feature = "replay"))]
use bento::kernel::cpu::*;
#[cfg(feature = "replay")]
use scheduler_utils;
#[cfg(feature = "replay")]
use scheduler_utils::spin_rs::RwLock;
#[cfg(feature = "replay")]
use scheduler_utils::cpu::*;

use serde::{Serialize, Deserialize};

use self::scheduler_utils::*;


use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;

use core::mem;
use core::str;
use core::fmt::Debug;

use RQLockGuard;
use self::ringbuffer::RingBuffer;
use self::sched_core::resched_cpu;

#[repr(C)]
pub struct BentoSched {
    pub map: Option<RwLock<BTreeMap<u64, Schedulable>>>,
    pub map2: Option<RwLock<BTreeMap<u64, RwLock<Option<Schedulable>>>>>,
    pub state: Option<RwLock<BTreeMap<u64, ProcessState>>>,
    pub state2: Option<RwLock<BTreeMap<u64, RwLock<Option<ProcessState>>>>>,
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

pub struct CpuState {
    pub weight: u64,
    pub inv_weight: u64,
    pub curr: Option<u64>,
    pub set: BTreeSet<(u64, u64)>,
    pub free_time: u64,
    pub load: u8,
    pub capacity: u8,
}

pub struct UpgradeData {
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
    fn add_weight(&self, cpu_state: &mut CpuState, prio: i32) {
        let weight = sched_core::SCHED_PRIO_TO_WEIGHT[(prio - 100) as usize] as u64;
        cpu_state.weight += weight;
        let weight_shift = cpu_state.weight >> 10;
        if weight_shift > 0 {
            let inv_weight = 0xffffffff / weight_shift;
            cpu_state.inv_weight = inv_weight;
        } else {
            cpu_state.inv_weight = 0xffffffff;
        }
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
    }
}

impl BentoScheduler<'_, '_, UpgradeData, UpgradeData, UserMessage, UserMessage> for BentoSched {
    fn get_policy(&self) -> i32 {
        10
    }

    fn task_new(&self, pid: u64, _tgid: u64, runtime: u64, runnable: u16, prio: i32, sched: Schedulable, _guard: RQLockGuard) {
        let mut vruntime = calculate_vruntime(runtime, prio);
        if runnable > 0 {
            let cpu;
            {
                let mut map = self.map2.as_ref().unwrap().write();
                if map.get(&pid).is_none() {
                    map.insert(pid, RwLock::new(Some(sched)));
                }
                cpu = map.get(&pid).unwrap().read().as_ref().unwrap().get_cpu();
            }
            {
                let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
                let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
                let real_set = &mut cpu_state.set;
                if let Some((min_vruntime, _)) = real_set.first() {
                    vruntime = core::cmp::max(vruntime, *min_vruntime);
                }
                real_set.insert((vruntime, pid));
                self.add_weight(&mut cpu_state, prio);
            }
            let mut state = self.state2.as_ref().unwrap().write();
            let procstate = ProcessState {
                prio: prio,
                vruntime: vruntime,
                last_runtime: runtime,
                cpu: cpu,
            };
            state.insert(pid, RwLock::new(Some(procstate)));
        } else {
            let cpu = sched.get_cpu();
            let mut map = self.map2.as_ref().unwrap().write();
            if map.get(&pid).is_none() {
                map.insert(pid, RwLock::new(Some(sched)));
            }
            let mut state = self.state2.as_ref().unwrap().write();
            let procstate = ProcessState {
                prio: prio,
                vruntime: vruntime,
                last_runtime: runtime,
                cpu: cpu,
            };
            state.insert(pid, RwLock::new(Some(procstate)));
        }
    }

    fn task_prio_changed(&self, pid: u64, prio: i32, _guard: RQLockGuard) {
        let old_prio;
        {
            let state = self.state2.as_ref().unwrap().read();
            let mut procstate_opt = state.get(&pid).unwrap().write();
            let mut procstate = procstate_opt.as_mut().unwrap();
            old_prio = procstate.prio;
            procstate.prio = prio;
        }
        let cpu;
        {
            let map = self.map2.as_ref().unwrap().read();
            cpu = map.get(&pid).unwrap().read().as_ref().unwrap().get_cpu();
        }
        {
            let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
            let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
            self.remove_weight(&mut cpu_state, old_prio);
            self.add_weight(&mut cpu_state, prio);
        }
    }

    fn task_wakeup(&self, pid: u64 _deferrable: bool,
                   _last_run_cpu: i32, _wake_up_cpu: i32, _waker_cpu: i32,
                   sched: Schedulable, _guard: RQLockGuard) {
        let cpu = sched.get_cpu();
        {
            let map = self.map2.as_ref().unwrap().read();
            let mut sched_val = map.get(&pid).unwrap().write();
            sched_val.replace(sched);
        }

        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
        if let Some(curr_pid) = cpu_state.curr {
            if curr_pid == pid {
                return;
            }
        }
        let real_set = &mut cpu_state.set;
        let state = self.state2.as_ref().unwrap().read();
        let mut procstate_opt = state.get(&pid).unwrap().write();
        let mut proc_state = procstate_opt.as_mut().unwrap();
        if let Some((min_vruntime, _)) = real_set.first() {
            if *min_vruntime > 6000000 {
                proc_state.vruntime = core::cmp::max(*min_vruntime - 6000000, proc_state.vruntime);
            }
        }
        proc_state.cpu = cpu;
        real_set.insert((proc_state.vruntime, pid));
        self.add_weight(&mut cpu_state, proc_state.prio);
    }

    fn task_preempt(&self, pid: u64, runtime: u64, _cpu_seqnum: u64, _cpu: i32,
                    _from_switchto: i8, _was_latched: i8, sched: Schedulable, _guard: RQLockGuard) {
        let vruntime;
        let old_vruntime;
        let cpu = sched.get_cpu();
        {
            let map = self.map2.as_ref().unwrap().read();
            let mut sched_val = map.get(&pid).unwrap().write();
            sched_val.replace(sched);
        }
        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
        let real_set = &mut cpu_state.set;
        {
            let state = self.state2.as_ref().unwrap().read();
            let mut procstate_opt = state.get(&pid).unwrap().write();
            let mut procstate = procstate_opt.as_mut().unwrap();
            old_vruntime = procstate.vruntime;
            let delta_runtime = runtime - procstate.last_runtime;
            procstate.vruntime += calculate_vruntime(delta_runtime, procstate.prio);
            vruntime = procstate.vruntime;
            procstate.last_runtime = runtime;
            procstate.cpu = cpu;
        }
        let remove_val = (old_vruntime, pid);
        real_set.remove(&remove_val);
        real_set.insert((vruntime, pid));
        if cpu_state.curr == Some(pid) {
            cpu_state.curr = None;
        }
    }

    fn task_blocked(&self, pid: u64, runtime: u64, _cpu_seqnum: u64,
                    cpu: i32, _from_switchto: i8, _guard: RQLockGuard) {
        let old_vruntime;
        let prio;
        {
            let state = self.state2.as_ref().unwrap().read();
            let mut procstate_opt = state.get(&pid).unwrap().write();
            let mut procstate = procstate_opt.as_mut().unwrap();
            old_vruntime = procstate.vruntime;
            prio = procstate.prio;
            let delta_runtime = runtime - procstate.last_runtime;
            procstate.vruntime += calculate_vruntime(delta_runtime, procstate.prio);
            procstate.last_runtime = runtime;
            procstate.cpu = cpu as u32;
        }
        let remove_val = (old_vruntime, pid);

        {
            let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
            let mut cpu_state = all_cpu_state.get(&(cpu as u32)).unwrap().write();
            let real_set = &mut cpu_state.set;
            if real_set.remove(&remove_val) {
                self.remove_weight(&mut cpu_state, prio);
            }
            if cpu_state.curr == Some(pid) {
                cpu_state.curr = None;
                self.remove_weight(&mut cpu_state, prio);
            }

        }
    }

    fn task_dead(&self, pid: u64, _guard: RQLockGuard) {
        let prio;
        let old_vruntime;
        {
            let mut state = self.state2.as_ref().unwrap().write();
            let proc_val = state.get_mut(&pid).unwrap();
            let mut procstate_opt = proc_val.write();
            let procstate = procstate_opt.as_mut().unwrap();
            old_vruntime = procstate.vruntime;
            prio = procstate.prio;
        }
        let remove_val = (old_vruntime, pid);

        let cpu;
        {
            let mut map = self.map2.as_ref().unwrap().write();
            cpu = map.get(&pid).unwrap().read().as_ref().unwrap().get_cpu();
            map.remove(&pid);
        }
        {
            let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
            let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
            let real_set = &mut cpu_state.set;
            if real_set.remove(&remove_val) {
                self.remove_weight(&mut cpu_state, prio);
            }
            if cpu_state.curr == Some(pid) {
                cpu_state.curr = None;
                self.remove_weight(&mut cpu_state, prio);
            }

        }
    }

    fn task_departed(
        &self,
        pid: u64,
        _cpu_seqnum: u64,
        _cpu: i32,
        _from_switchto: i8,
        _was_current: i8,
        _guard: RQLockGuard
    ) -> Schedulable {
        let prio;
        let old_vruntime;
        {
            let mut state = self.state2.as_ref().unwrap().write();
            let proc_val = state.get_mut(&pid).unwrap();
            let mut procstate_opt = proc_val.write();
            let procstate = procstate_opt.as_mut().unwrap();
            old_vruntime = procstate.vruntime;
            prio = procstate.prio;
        }
        let remove_val = (old_vruntime, pid);

        let cpu;
        let old_sched = {
            let mut map = self.map2.as_ref().unwrap().write();
            cpu = map.get(&pid).unwrap().read().as_ref().unwrap().get_cpu();
            map.remove(&pid).unwrap().write().take().unwrap()
        };
        {
            let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
            let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
            let real_set = &mut cpu_state.set;
            if real_set.remove(&remove_val) {
                self.remove_weight(&mut cpu_state, prio);
            }
            if cpu_state.curr == Some(pid) {
                cpu_state.curr = None;
                self.remove_weight(&mut cpu_state, prio);
            }

        }
        return old_sched;
    }

    fn pick_next_task(&self, cpu: i32, curr_sched: Option<Schedulable>,
                      curr_runtime: Option<u64>, _guard: RQLockGuard) -> Option<Schedulable> {
        {
        let curr_pid_opt = if let Some(current) = curr_sched {
            let pid = current.get_pid();
            let map = self.map2.as_ref().unwrap().read();
            let mut sched_val = map.get(&current.get_pid()).unwrap().write();
            sched_val.replace(current);
            Some(pid)
        } else {
            None
        };
        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&(cpu as u32)).unwrap().write();
        let curr_reintro = if let Some(runtime) = curr_runtime &&
            let Some(curr_pid) = curr_pid_opt &&
            let Some(curr_pid2) = cpu_state.curr &&
            curr_pid == curr_pid2 {
            let state = self.state2.as_ref().unwrap().read();
            let mut procstate_opt = state.get(&curr_pid).unwrap().write();
            let mut procstate = procstate_opt.as_mut().unwrap();
            let delta_runtime = runtime - procstate.last_runtime;
            procstate.vruntime += calculate_vruntime(delta_runtime, procstate.prio);
            procstate.last_runtime = runtime;
            Some((procstate.vruntime, curr_pid))
        } else {
            None
        };
        let set = &mut cpu_state.set;
        if let Some(curr_val) = curr_reintro {
            set.insert(curr_val);
        }
        if let Some((_vruntime, pid)) = set.pop_first() {
            let sched_opt = {
                let map = self.map2.as_ref().unwrap().read();
                let sched_val = map.get(&pid).unwrap().write().take();
                sched_val
            };
            if sched_opt.is_none() {
                println!("Terrible problem, pid has no cpu");
                return None;
            }
            if sched_opt.as_ref().unwrap().get_cpu() == cpu as u32 ||
                sched_opt.as_ref().unwrap().get_cpu() == u32::MAX {
                let num_waiting = set.len();
                if num_waiting >= 1 {
                    let period: u64 = if num_waiting > 8 {
                        (num_waiting as u64) * 750000
                    } else {
                        6000000
                    };
                    let weight;
                    let inv_weight;
                    {
                        let state = self.state2.as_ref().unwrap().read();
                        let procstate_opt = state.get(&pid).unwrap().read();
                        let procstate = procstate_opt.as_ref().unwrap();
                        let prio = procstate.prio;
                        weight = sched_core::SCHED_PRIO_TO_WEIGHT[(prio - 100) as usize] as u64;
                        inv_weight = cpu_state.inv_weight;
                    }
                    let weight_shift = weight >> 10;
                    let factor = weight_shift * inv_weight;
                    let slice_calc = period * factor;
                    let slice = slice_calc >> 32;
                    hrtick::hrtick_start(cpu, slice);
                }
                cpu_state.curr = Some(sched_opt.as_ref().unwrap().get_pid());
                cpu_state.load = cpu_state.load << 1;
                cpu_state.capacity = cpu_state.capacity << 1;
                if num_waiting >= 1 {
                    cpu_state.load = cpu_state.load | 1;
                }
                return Some(sched_opt.unwrap());
            } else {
                cpu_state.curr = None;
                return None;
            }
        } else {
        }
        cpu_state.load = cpu_state.load << 1;
        cpu_state.capacity = cpu_state.capacity << 1;
        cpu_state.capacity = cpu_state.capacity | 1;
        cpu_state.curr = None;
        return None;
        }
    }

    fn pnt_err(&self, cpu: i32, pid: u64, err: i32, _sched: Option<Schedulable>, _guard: RQLockGuard) {
        if err == 2 {
            let sched = _sched.unwrap();
            let map = self.map2.as_ref().unwrap().read();
            let mut sched_val = map.get(&sched.get_pid()).unwrap().write();
            sched_val.replace(sched);
        }
        let prio;
        {
            let state = self.state2.as_ref().unwrap().read();
            let procstate_opt = state.get(&pid).unwrap().read();
            let procstate = procstate_opt.as_ref().unwrap();
            prio = procstate.prio;
        }
        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&(cpu as u32)).unwrap().write();
        cpu_state.curr = None;
        if err == 1 {
            self.remove_weight(&mut cpu_state, prio);
        }
    }

    fn balance_err(&self, _cpu: i32, pid: u64, _err: i32, _sched: Option<Schedulable>, _guard: RQLockGuard) {
        let mut balancing = self.balancing.as_ref().unwrap().write();
        balancing.remove(&pid);
    }

    fn select_task_rq(&self, pid: u64, _waker_cpu: i32, _prev_cpu: i32) -> i32 {
        let state = self.state2.as_ref().unwrap().read();
        match state.get(&pid) {
            None => {
                pid as i32 % num_online_cpus()
            },
            Some(proc_lock) => {
                let procstate_opt = proc_lock.read();
                let procstate = procstate_opt.as_ref().unwrap();
                procstate.cpu as i32
            },
        }
    }

    fn migrate_task_rq(&self, pid: u64, sched: Schedulable, _guard: RQLockGuard) -> Schedulable {
        let old_cpu;
        let cpu = sched.get_cpu();
        let old_sched;
        {
            let map = self.map2.as_ref().unwrap().read();
            let mut sched_val = map.get(&pid).unwrap().write();
            old_sched = sched_val.replace(sched).unwrap();
            old_cpu = old_sched.get_cpu();
        }
        let vruntime;
        let prio;
        let runtime;
        {
            let state = self.state2.as_ref().unwrap().read();
            let mut procstate_opt = state.get(&pid).unwrap().write();
            let mut procstate = procstate_opt.as_mut().unwrap();
            vruntime = procstate.vruntime;
            runtime = procstate.last_runtime;
            prio = procstate.prio;
            procstate.cpu = cpu;
        }
        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        if old_cpu != u32::MAX {
            let mut cpu_state = all_cpu_state.get(&old_cpu).unwrap().write();
            let real_set = &mut cpu_state.set;
            real_set.remove(&(vruntime, pid));
            self.remove_weight(&mut cpu_state, prio);
            cpu_state.free_time = 0;
            cpu_state.load = cpu_state.load << 1;
            if Some(pid) == cpu_state.curr {
                cpu_state.curr = None;
            }
        }
        let updated_vruntime;
        {
            let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
            let real_set = &mut cpu_state.set;
            let raw_vruntime = calculate_vruntime(runtime, prio);
            updated_vruntime = if vruntime > raw_vruntime {
                if let Some((min_vruntime, _)) = real_set.first() {
                    core::cmp::max(raw_vruntime, *min_vruntime)
                } else {
                    raw_vruntime
                }
            } else {
                vruntime
            };
            real_set.insert((updated_vruntime, pid));
            self.add_weight(&mut cpu_state, prio);
            cpu_state.free_time = 0;
            cpu_state.capacity = cpu_state.capacity << 1;
        }
        if updated_vruntime != vruntime {
            let state = self.state2.as_ref().unwrap().read();
            let mut procstate_opt = state.get(&pid).unwrap().write();
            let mut procstate = procstate_opt.as_mut().unwrap();
            procstate.vruntime = updated_vruntime;
        }
        let mut balancing = self.balancing.as_ref().unwrap().write();
        balancing.remove(&pid);
        return old_sched;
    }

    fn task_affinity_changed(&self, pid: u64, _cpumask: u64) {
        let mut locked = self.locked.as_ref().unwrap().write();
        locked.insert(pid);
    }

    fn balance(&self, cpu: i32, _guard: RQLockGuard) -> Option<u64> {
        let cpu_state = self.cpu_state.as_ref().unwrap().read();
        if cpu_state.get(&(cpu as u32)).is_none() {
        } else if let Some(lock) = cpu_state.get(&(cpu as u32)) {
            let state = lock.read();
            let idle = state.curr.is_none();
            let set = &state.set;
            if (set.is_empty() && idle) || state.capacity.count_ones() >= num_online_cpus() as u32 {
                let mut max_tasks = 0;
                let mut balance_pid = None;
                for i in 0..num_online_cpus() {
                    if i == cpu {
                        continue;
                    }
                    if let Some(lock2) = cpu_state.get(&(i as u32)) {
                        let state2 = lock2.read();
                        let idle2 = state2.curr.is_none();
                        let set2 = &state2.set;
                        if set2.len() > max_tasks && !(idle2 && set2.len() == 1) && state2.capacity.count_ones() < 4 {
                            for (_, pid) in set2 {
                                let mut balancing = self.balancing.as_ref().unwrap().write();
                                let locked = self.locked.as_ref().unwrap().read();
                                if Some(*pid) != state2.curr && !balancing.contains(pid) && !locked.contains(pid) {
                                    max_tasks = set2.len();
                                    if let Some(old_pid) = balance_pid {
                                        balancing.remove(&old_pid);
                                    }
                                    balance_pid = Some(*pid);
                                    balancing.insert(*pid);
                                    break;
                                }
                            }
                        }
                    }
                }
                return balance_pid;
            }
        }
        return None;
    }

    fn task_yield(
        &self, pid: u64, runtime: u64,
        _cpu_seqnum: u64, _cpu: i32, _from_switchto: i8, sched: Schedulable, _guard: RQLockGuard
    ) {
        let cpu = sched.get_cpu();
        {
            let map = self.map2.as_ref().unwrap().write();
            let mut sched_val = map.get(&pid).unwrap().write();
            sched_val.replace(sched);
        }
        let state = self.state2.as_ref().unwrap().read();
        let mut procstate_opt = state.get(&pid).unwrap().write();
        let mut proc_state = procstate_opt.as_mut().unwrap();
        let old_vruntime = proc_state.vruntime;
        let delta_runtime = runtime - proc_state.last_runtime;
        proc_state.vruntime += calculate_vruntime(delta_runtime, proc_state.prio);
        proc_state.last_runtime = runtime;
        proc_state.cpu = cpu;

        let all_cpu_state = self.cpu_state.as_ref().unwrap().read();
        let mut cpu_state = all_cpu_state.get(&cpu).unwrap().write();
        let real_set = &mut cpu_state.set;
        let remove_val = (old_vruntime, pid);
        real_set.remove(&remove_val);
        real_set.insert((proc_state.vruntime, pid));
        if cpu_state.curr == Some(pid) {
            cpu_state.curr = None;
        }
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

    fn register_queue(&self, _pid: u64, q: RingBuffer<UserMessage>) -> i32 {
        let mut user_q = self.user_q.as_ref().unwrap().write();
        let next = user_q.keys().max().map_or(0, |max| max + 1);
        user_q.insert(next as i32, q);
        return next as i32;
    }

    fn register_reverse_queue(&self, _pid: u64, q: RingBuffer<UserMessage>) -> i32 {
        let mut rev_q = self.rev_q.as_ref().unwrap().write();
        let next = rev_q.keys().max().map_or(0, |max| max + 1);
        rev_q.insert(next as i32, q);
        return next as i32;
    }

    fn unregister_queue(&self, id: i32) -> RingBuffer<UserMessage> {
        let mut user_q = self.user_q.as_ref().unwrap().write();
        let q = user_q.remove(&id);
        q.unwrap()
    }

    fn unregister_rev_queue(&self, id: i32) -> RingBuffer<UserMessage> {
        let mut rev_q = self.rev_q.as_ref().unwrap().write();
        let q = rev_q.remove(&id);
        q.unwrap()
    }

    fn task_tick(&self, cpu: i32, queued: bool, guard: RQLockGuard) {
        if queued {
            resched_cpu(cpu, &guard);
        }
    }
}

