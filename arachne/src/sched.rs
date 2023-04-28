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
//
use sched_core::resched_cpu;

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::BTreeSet;

use core::mem;
use core::str;
use core::fmt::Debug;

use self::ringbuffer::RingBuffer;

#[repr(C)]
pub struct BentoSched {
    //pub q: Option<RwLock<VecDeque<u64>>>,
    pub q: Option<RwLock<VecDeque<u64>>>,
    pub qs: Option<RwLock<BTreeMap<u32,RwLock<VecDeque<u64>>>>>,
    pub map: Option<RwLock<BTreeMap<u64, Schedulable>>>,
    pub map2: Option<RwLock<BTreeMap<u64, RwLock<Option<Schedulable>>>>>,
    pub user_q: Option<RwLock<BTreeMap<i32, RingBuffer<UserMessage>>>>,
    pub rev_q: Option<RwLock<BTreeMap<i32, RingBuffer<RevMessage>>>>,
    pub core_requests: Option<RwLock<BTreeMap<u64, UserMessage>>>,
    pub core_map: Option<RwLock<BTreeMap<u32, u64>>>,
    pub proc_pids: Option<RwLock<BTreeMap<u64, RwLock<BTreeMap<u64, u32>>>>>,
    pub pid_state: Option<RwLock<BTreeMap<u64, ProcessState>>>,
    pub moved: Option<RwLock<BTreeSet<u64>>>,
    pub need_timer: Option<RwLock<BTreeSet<u32>>>,
    pub must_balance: Option<RwLock<BTreeSet<u64>>>,
    pub evicting: Option<RwLock<BTreeMap<u32, (u64, i32)>>>,
    pub assigned: Option<RwLock<BTreeMap<u32, u64>>>,
    pub cpu_running: Option<RwLock<BTreeSet<i32>>>,
    pub clearing: Option<RwLock<bool>>,
    //pub need_clear: Option<RwLock<BTreeMap<u64, i32>>>,
}

pub struct UpgradeData {
    q: Option<RwLock<VecDeque<u64>>>,
    map: Option<RwLock<BTreeMap<u64, Schedulable>>>
}

pub struct ProcessState {
    cpu: u32,
    tgid: u64
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct UserMessage {
    tgid: u64,
    prio0: u32,
    prio1: u32,
    prio2: u32,
    prio3: u32,
    prio4: u32,
    prio5: u32,
    prio6: u32,
    prio7: u32,
}

#[repr(C)]
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct RevMessage {
    reclaim: bool
}

impl BentoScheduler<'_, '_, UpgradeData, UpgradeData, UserMessage, RevMessage> for BentoSched {
    fn get_policy(&self) -> i32 {
        10
    }

    //fn init(&mut self) {
   // }

    fn task_new(&self, pid: u64, tgid: u64, _runtime: u64, runnable: u16, _prio: i32, 
                sched: Schedulable, _guard: RQLockGuard) {
        //println!("tid in task_new: {} tgid {} {}", pid, tgid, sched.get_cpu());
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();
        if runnable > 0 {
            //q.push_back(pid);
        }
        let mut map = self.map2.as_ref().unwrap().write();
        let cpu = sched.get_cpu();
        if map.get(&pid).is_none() {
            map.insert(pid, RwLock::new(Some(sched)));
        }
        let mut proc_pids = self.proc_pids.as_ref().unwrap().write();
        if let Some(proc_lock) = proc_pids.get(&tgid) {
            let mut proc_map = proc_lock.write();
            proc_map.insert(pid, cpu as u32);
        } else {
            let mut proc_map = BTreeMap::new();
            proc_map.insert(pid, cpu as u32);
            proc_pids.insert(tgid, RwLock::new(proc_map));
        }
        let mut pid_state = self.pid_state.as_ref().unwrap().write();
        pid_state.insert(pid, ProcessState{cpu: cpu, tgid: tgid});
    }

    fn task_wakeup(&self, pid: u64, _agent_data: u64, deferrable: bool,
                   _last_run_cpu: i32, _wake_up_cpu: i32, _waker_cpu: i32,
                   sched: Schedulable, _guard: RQLockGuard) {
        //println!("wakeup {}", pid);
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();
        //let mut q = self.q.as_ref().unwrap().write();
        let mut map = self.map2.as_ref().unwrap().read();
        if deferrable {
            //q.push_back(pid);
        } else {
            //q.push_front(pid);
        }
        //map.insert(pid, wake_up_cpu as u32);
        let mut sched_val = map.get(&pid).unwrap().write();
        sched_val.replace(sched);
        //map.insert(pid, sched);
    }

    fn task_preempt(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64, _cpu: i32,
                    _from_switchto: i8, _was_latched: i8, sched: Schedulable, _guard: RQLockGuard) {
        //println!("preempt {}", pid);
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();
        //let mut q = self.q.as_ref().unwrap().write();
        //q.push_back(pid);
        let mut map = self.map2.as_ref().unwrap().read();
        let mut sched_val = map.get(&pid).unwrap().write();
        sched_val.replace(sched);
        //map.insert(pid, sched);
    }

    fn task_blocked(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64,
                    cpu: i32, _from_switchto: i8, _guard: RQLockGuard) {
        //println!("blocked {}", pid);
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&(cpu as u32)).unwrap().write();
        //let mut q = self.q.as_ref().unwrap().write();
        // position depends on iterator state, so don't use iterator before we call position
        if let Some(idx) = q.iter().position(|&x| x == pid) {
            //q.remove(idx);
        }
        let mut map = self.map2.as_ref().unwrap().write();
        let sched_lock = map.get(&pid).unwrap();
        let mut sched_val = sched_lock.write();
        let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
        cpu_running.remove(&cpu);
        sched_val.take();
        //map.remove(&pid);
        //map.insert(pid, cpu as u32);
        //map.insert(pid, sched);
    }

    fn task_yield(
        &self, pid: u64, runtime: u64,
        _cpu_seqnum: u64, cpu: i32, _from_switchto: i8, sched: Schedulable, _guard: RQLockGuard
    ) {
       // println!("yielding {} on cpu {}", pid, cpu);
        // TODO: setup mapping from parent pid to which core this guy is on
        //let mut map = self.map.as_ref().unwrap().write();
        {
        let mut clearing = self.clearing.as_ref().unwrap().write();
        let mut assigned = self.assigned.as_ref().unwrap().write();
        let mut core_map = self.core_map.as_ref().unwrap().write();
        let mut evicting = self.evicting.as_ref().unwrap().write();
        let mut should_remove = None;
        for evict_core in evicting.keys() {
            if let Some(this_tgid) = core_map.get(&(cpu as u32)) {
                if Some(this_tgid) == core_map.get(evict_core) {
                    //println!("updating in yield");
                    if let Some(ass_pid) = assigned.get(&(cpu as u32)) && *ass_pid == pid {
                        assigned.remove(&(cpu as u32));
                    }
                    core_map.insert(cpu as u32, 0);
                    should_remove = Some(*evict_core);
                    *clearing = false;
                }
            }
            //if core_map.get(&(cpu as u32)).is_some() && core_map.get(&(cpu as u32)) == core_map
        }
        if let Some(evict_core) = should_remove {
            evicting.remove(&evict_core);
        }
        }
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();
        let map = self.map2.as_ref().unwrap().read();
        let sched_lock = map.get(&pid).unwrap();
        let mut sched_val = sched_lock.write();

        //for tgid in clearing {
        //    if proc_pids.get(tgid).contains(&pid) {
        //        clearning.remove(&tgid);
        //        need_clear.clear();
        //    }
        //}
        //let mut q = self.q.as_ref().unwrap().write();
        //let mut pid_state = self.pid_state.as_ref().unwrap().write();
        if !q.contains(&pid) {
            //q.push_back(pid);
        }
        //pid_state.insert(pid, sched.get_cpu());
        sched_val.replace(sched);
        // TODO: if evicting anything else for the tgid, also remove that
        //let mut evicting = self.evicting.as_ref().unwrap().write();
        //evicting.remove(&(cpu as u32));
    }

    fn pick_next_task(&self, cpu: i32, curr_sched: Option<Schedulable>, curr_runtime: Option<u64>, _guard: RQLockGuard) -> Option<Schedulable> {
        if curr_sched.is_some() {
            let cpu_running = self.cpu_running.as_ref().unwrap().read();
            if cpu_running.contains(&cpu) {
            //println!("got curr {:?}", curr_sched);
            } else {
           //     println!("got fake curr {:?}", curr_sched);
            }
        }
        {
            let mut need_timer = self.need_timer.as_ref().unwrap().write();
            if need_timer.contains(&(cpu as u32)) {
                //println!("pick clearing need timer");
                need_timer.remove(&(cpu as u32));
            }
        }
        if cpu == 0 {
            let qs = self.qs.as_ref().unwrap().read();
            let mut q = qs.get(&(cpu as u32)).unwrap().write();
            if let Some(pid) = q.get(0) {
                let mut map = self.map2.as_ref().unwrap().read();
                let mut sched_val = map.get(&pid).unwrap().write();
                if sched_val.is_none() {
                    if let Some(ref curr) = curr_sched && curr.get_pid() == *pid {
                        //println!("would shortcut {} core {}", pid, cpu);
                        //return curr_sched;
                    }
                    //println!("Terrible problem, pid has no cpu {}", cpu);
                    let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
                    cpu_running.remove(&cpu);
                    return None;
                }
                if (sched_val.as_ref().unwrap().get_cpu() == cpu as u32) {
                    let sched = sched_val.take();
                    //println!("would pick {} core {}", pid, cpu);
                    let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
                    cpu_running.insert(cpu);
                    //return None;
                    hrtick::hrtick_start(cpu, 1000000000);
                    //hrtick::hrtick_start(cpu, 10000000);
                    return sched;
                }
            }
        }
        let mut assigned = self.assigned.as_ref().unwrap().write();
        if let Some(pid) = assigned.get(&(cpu as u32)) {
            let mut map = self.map2.as_ref().unwrap().read();
            let mut sched_val = map.get(&pid).unwrap().write();
            if sched_val.is_none() {
                if let Some(ref curr) = curr_sched && curr.get_pid() == *pid {
                    //println!("would shortcut {} core {}", pid, cpu);
                    //return curr_sched;
                }
                //println!("Terrible problem, pid has no cpu {}", cpu);
                let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
                cpu_running.remove(&cpu);
                return None;
            }
            if (sched_val.as_ref().unwrap().get_cpu() == cpu as u32) {
                let sched = sched_val.take();
                //println!("would pick {} core {}", pid, cpu);
                //return None;
                let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
                cpu_running.insert(cpu);
                hrtick::hrtick_start(cpu, 1000000000);
                //hrtick::hrtick_start(cpu, 10000000);
                return sched;
            }
        }
        let core_map = self.core_map.as_ref().unwrap().read();
        if core_map.get(&(cpu as u32)).is_none() || core_map.get(&(cpu as u32)).unwrap() == &0 {
            let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
            cpu_running.remove(&cpu);
            //println!("nothing in core map {}", cpu);
            return None;
        }
        let tgid = core_map.get(&(cpu as u32)).unwrap();
        let proc_pids = self.proc_pids.as_ref().unwrap().read();
        if proc_pids.get(&tgid).is_none() {
            // Nothing to pull
            //println!("nothing to pull {}", cpu);
            let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
            cpu_running.remove(&cpu);
            return None;
        }
        let proc_lock = proc_pids.get(&tgid).unwrap();
        let proc_cores = proc_lock.read();
        if let Some((pid, core)) = proc_cores.iter().find(|(&pid, &core)| core == cpu as u32) {

            let mut map = self.map2.as_ref().unwrap().read();
            let mut sched_val = map.get(&pid).unwrap().write();
            if sched_val.is_none() {
                if let Some(ref curr) = curr_sched && curr.get_pid() == *pid {
                    //println!("would shortcut {} core {}", pid, cpu);
                    //return curr_sched;
                }
                //println!("Terrible problem, pid has no cpu {}", cpu);
                let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
                cpu_running.remove(&cpu);
                return None;
            }
            if (sched_val.as_ref().unwrap().get_cpu() == cpu as u32) {
                let sched = sched_val.take();
                assigned.insert(cpu as u32, *pid);
                //println!("would pick {} core {}", pid, cpu);
                //return None;
                let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
                cpu_running.insert(cpu);
                hrtick::hrtick_start(cpu, 1000000000);
                //hrtick::hrtick_start(cpu, 10000000);
                return sched;
            }
        }
        let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
        cpu_running.remove(&cpu);
        //println!("truly nothing {}", cpu);
        return None;
        //let qs = self.qs.as_ref().unwrap().read();
        //let mut q = qs.get(&(cpu as u32)).unwrap().write();
        ////let mut q = self.q.as_ref().unwrap().write();
        //// let map = self.map.as_ref().unwrap().read();
        //
        //// let mut idx : usize = 0usize;
        //// while idx < q.len() {
        //    // // cycle through the queue until we find first task belonging to this CPU
        //    // if let Some(pid) = q.get(idx) {
        //        // if map.get(&pid).is_some() {
        //            // // if this task's cpu matches current target cpu, use this task
        //            // if (map.get(&pid).unwrap().get_cpu() == cpu as u32 ||
        //                // map.get(&pid).unwrap().get_cpu() == u32::MAX) {
        //                // break;
        //            // }
        //        // }
        //    // }
        //    // idx += 1;
        //// }

        //// if (idx >= q.len()) {
        //    // return None;
        //// }

        //// if let Some(pid) = q.remove(idx) {
        //    // hrtick::hrtick_start(cpu, 10000);
        //    // return Some(*map.get(&pid).unwrap());
        //// }

        //// return None;

        //let mut idx : usize = 0usize;
        //while idx < q.len() {
        ////if let Some(pid) = q.pop_front() {
        //    // TODO: change this to write() or read() ?
        //    let pid = q.get(idx).unwrap();
        //    let mut map = self.map2.as_ref().unwrap().read();
        //    let mut sched_val = map.get(&pid).unwrap().write();
        //    if sched_val.is_none() {
        //        println!("Terrible problem, pid has no cpu");
        //        //q.push_front(pid);
        //        return None;
        //    }
        //    if (sched_val.as_ref().unwrap().get_cpu() == cpu as u32 ||
        //        sched_val.as_ref().unwrap().get_cpu() == u32::MAX) {

        //        //println!("pnt cpu {} would pick {}", cpu, pid);
        //        //return None;
        //        //Some(pid)
        //        // None shouldn't happen anymore
        //        // TODO: probably should turn this into a constant
        //        // hrtick::hrtick_start(cpu, 10000);
        //        //Some(*map.get(&pid).unwrap())
        //        // TODO: I hope this works...
        //        //let sched = map.remove(&pid);
        //        let sched = sched_val.take();
        //        q.remove(idx);
        //        return sched;
        //    }
        //        //println!("can't schedule on other cpu\n");
        //        //q.push_front(pid);
        //    //    None
        //    //}
        //    idx += 1;
        //}
        //None
    }

    fn pnt_err(&self, cpu: i32, pid: u64, err: i32, _sched: Option<Schedulable>, _guard: RQLockGuard) {
        println!("pnt_err");
        // TODO: I think we can just comment this out in pnt_err?
        //let mut map = self.map.as_ref().unwrap().write();
        //map.insert(sched.get_pid(), sched);
    }

    fn select_task_rq(&self, pid: u64, _waker_cpu: i32, _prev_cpu: i32) -> i32 {
        //println!("tid in select_task_rq: {}", pid);
        //let map = self.map2.as_ref().unwrap().read();
        //*map.get(&pid).unwrap_or(&1) as i32
        // TODO: remember if we've moved something
        let pid_state = self.pid_state.as_ref().unwrap().read();
        //let cpu = sched.get_cpu();
        let retval = match pid_state.get(&pid) {
            None => pid as i32 % 6,
            Some(state) => state.cpu as i32,
        };
        //println!("selecting {} for pid {}", retval, pid);
        return retval;

        // match map.get(&pid) {
            //None => {
            //    //map.insert(pid, pid as u32 % 2);
            //    pid as i32 % 2
            //},
            // None => 1,
            // Some(sched) => sched.get_cpu() as i32,
        // }
    }

    fn selected_task_rq(&self, sched: Schedulable) {
        //println!("selected");
        //let mut map = self.map2.as_ref().unwrap().read();
        //let mut sched_val = map.get(&sched.get_pid()).unwrap().write();
        //sched_val.replace(sched);
        //map.insert(sched.get_pid(), sched);
    }

    //fn migrate_task_rq(&self, pid: u64, new_cpu: i32) {
    fn migrate_task_rq(&self, pid: u64, sched: Schedulable, _guard: RQLockGuard) -> Schedulable {
        //println!("hitting migrate_task_rq pid {} to cpu {}", pid, sched.get_cpu());
        let qs = self.qs.as_ref().unwrap().read();
        let mut map = self.map2.as_ref().unwrap().read();
        let mut sched_val = map.get(&pid).unwrap().write();

        let mut pid_state = self.pid_state.as_ref().unwrap().write();
        let proc_pids = self.proc_pids.as_ref().unwrap().read();
        //let cpu = sched.get_cpu();
        let state = pid_state.get_mut(&pid).unwrap();
        let proc_lock = proc_pids.get(&state.tgid).unwrap();
        state.cpu = sched.get_cpu();
        let mut moved = self.moved.as_ref().unwrap().write();
        moved.remove(&pid);
        let mut must_balance = self.must_balance.as_ref().unwrap().write();
        must_balance.remove(&pid);
        let mut proc_cores = proc_lock.write();
        proc_cores.insert(pid, sched.get_cpu());
        if sched.get_cpu() == 0 {
            let mut new_q = qs.get(&0).unwrap().write();
            new_q.push_back(pid);
        }
        if sched_val.is_some() {
            let mut assigned = self.assigned.as_ref().unwrap().write();
            let cpu = sched.get_cpu();
            let old_sched = sched_val.replace(sched);
            if let Some(ass_pid) = assigned.get(&old_sched.as_ref().unwrap().get_cpu()) && *ass_pid == pid {
                assigned.remove(&old_sched.as_ref().unwrap().get_cpu());
            }
            if old_sched.as_ref().unwrap().get_cpu() == 0 {
                let mut old_q = qs.get(&old_sched.as_ref().unwrap().get_cpu()).unwrap().write();
                //let mut new_q = qs.get(&cpu).unwrap().write();
                if let Some(idx) = old_q.iter().position(|&x| x == pid) {
                    old_q.remove(idx);
                }
            }
            //new_q.push_back(pid);
            return old_sched.unwrap();
        } else {
            // not runnable anyway, so don't worry, we'll fix it in wakeup or whatever.
            return sched;
        }
        //pid_state.insert(pid, cpu);
        //map.insert(pid, new_cpu as u32);
        //let old_sched = map.remove(&pid).unwrap();
        //map.insert(pid, sched);

    }

    fn balance(&self, cpu: i32, _guard: RQLockGuard) -> Option<u64> {
        // TODO: actually balance, but only when its time to shove a task off its core
        // if we want to schedule on the core, but nothing on the core is for that proc,
        // grab something from a different core
        if cpu == 0 {
            let must_balance = self.must_balance.as_ref().unwrap().read();
            if let Some(balance_pid) = must_balance.first() {
                //println!("balancing {} to {}", *balance_pid, cpu);
                return Some(*balance_pid);
            } else {
                return None;
            }
        }

        let core_map = self.core_map.as_ref().unwrap().read();
        if core_map.get(&(cpu as u32)).is_none() || core_map.get(&(cpu as u32)).unwrap() == &0 {
            if cpu == 1 {
                //println!("core not assigned");
            }
            return None;
        }
        let tgid = core_map.get(&(cpu as u32)).unwrap();
        let proc_pids = self.proc_pids.as_ref().unwrap().read();
        if proc_pids.get(&tgid).is_none() {
            // Nothing to pull
            if cpu == 1 {
                //println!("no pids");
            }
            return None;
        }
        let proc_lock = proc_pids.get(&tgid).unwrap();
        let proc_cores = proc_lock.read();
        if proc_cores.values().any(|&x| x == cpu as u32) {
            // Already something correct on the core
            if cpu == 1 {
                //println!("core ok");
            }
            return None;
        }
        // No tasks are ready on a core that needs to be scheduled
        let mut moved = self.moved.as_ref().unwrap().write();
        for (pid, core) in proc_cores.iter().rev() {
            if moved.contains(&pid) {
                continue;
            }
            let assigned = self.assigned.as_ref().unwrap().read();
            if assigned.get(&core) == Some(&pid) {
                continue;
            }
            if core_map.get(&core).is_none() || core_map.get(&core).unwrap() != tgid {
                //println!("balancing {} to {}", *pid, cpu);
                //return None;
                moved.insert(*pid);
                return Some(*pid);
            } else {
                if proc_cores.values().filter(|&x| x == core).count() > 1 {
                    //println!("balancing {} to {}", *pid, cpu);
                    //return None;
                    moved.insert(*pid);
                    return Some(*pid);
                }
            }
        }
        if cpu == 1 {
            //println!("no valid tasks");
        }
        return None;
    }

    fn balance_err(&self, _cpu: i32, pid: u64, err: i32, _sched: Option<Schedulable>, _guard: RQLockGuard) {
       // println!("balancing err");
        let mut moved = self.moved.as_ref().unwrap().write();
        moved.remove(&pid);
    }

    fn reregister_prepare(&mut self) -> Option<UpgradeData> {
        let mut data = UpgradeData {
            map: None,
            q: None
        };
        mem::swap(&mut data.map, &mut self.map);
        mem::swap(&mut data.q, &mut self.q);
        return Some(data);
    }

    fn reregister_init(&mut self, data_opt: Option<UpgradeData>) {
        if let Some(mut data) = data_opt {
            mem::swap(&mut self.map, &mut data.map);
            mem::swap(&mut self.q, &mut data.q);
        }
    }

    fn register_queue(&self, pid: u64, q: RingBuffer<UserMessage>) -> i32 {
        //println!("q ptr {:?}", q.inner);
        unsafe {
        println!("start registering new queue");
        //println!("q ptr {:?}", (*q.inner).offset);
        //println!("q capacity {}", (*q.inner).capacity);
        //println!("q readptr {}", (*q.inner).readptr);
        //println!("q writeptr {}", (*q.inner).writeptr);
        let mut user_q = self.user_q.as_ref().unwrap().write();
        let next = user_q.keys().max().map_or(0, |max| max + 1);
        //println!("Obtained current user queue {}", next);
        user_q.insert(next as i32, q);
        //println!("Replacing with passed-in queue");
        return next as i32;

        //self.user_q = Some(RwLock::new(q));
        }
    }

    fn register_reverse_queue(&self, pid: u64, q: RingBuffer<RevMessage>) -> i32 {
        //println!("q ptr {:?}", q.inner);
        unsafe {
        //println!("start registering new reverse queue");
        //println!("q ptr {:?}", (*q.inner).offset);
        //println!("q capacity {}", (*q.inner).capacity);
        //println!("q readptr {}", (*q.inner).readptr);
        //println!("q writeptr {}", (*q.inner).writeptr);
        let mut rev_q = self.rev_q.as_ref().unwrap().write();
        //let next = rev_q.keys().max().map_or(0, |max| max + 1);
        rev_q.insert(pid as i32, q);
        return pid as i32;

        //self.user_q = Some(RwLock::new(q));
        }
    }

    fn enter_queue(&self, id: i32, entries: u32) {
        //println!("Entries to poll {}, q {}", entries, id);
        // TODO: Figure out what the new setups should be
        let mut user_q_list = self.user_q.as_ref().unwrap().write();
        let user_q = user_q_list.get_mut(&id).unwrap();
        for i in 0..entries {
            let msg = user_q.dequeue().unwrap();
            //println!("msg val {:?}", &msg);
            let mut clearing = self.clearing.as_ref().unwrap().write();
            if *clearing {
                continue;
            }
            let mut core_requests = self.core_requests.as_ref().unwrap().write();
            core_requests.insert(msg.tgid, msg);

            // Decide how many cores each tgid should get
            let mut tgid_num_cores = BTreeMap::new();
            let mut total_assigned_cores = 0;
            for j in 0..6 {
                for (&tgid, &msg) in core_requests.iter() {
                    if !tgid_num_cores.contains_key(&tgid) {
                        tgid_num_cores.insert(tgid, 0);
                    }
                    let core_arr = [msg.prio0, msg.prio1, msg.prio2, msg.prio3, msg.prio4, msg.prio5, msg.prio6, msg.prio7];
                    let to_add = core::cmp::min(*core_arr.get(j).unwrap(), 6 - total_assigned_cores);
                    *tgid_num_cores.get_mut(&tgid).unwrap() += to_add;
                    total_assigned_cores += to_add;
                    if total_assigned_cores >= 6 {
                        break;
                    }
                }
                if total_assigned_cores >= 6 {
                    break;
                }
            }
            //println!("num_cores {:?}", tgid_num_cores);

            // Kick off procs from any tgids that should receive fewer cores
            let mut core_map = self.core_map.as_ref().unwrap().write();
            for (&tgid, &msg) in core_requests.iter() {
                let curr_assigned = core_map.values().filter(|&x| *x == tgid).count();
                let num_cores = tgid_num_cores.get(&tgid).unwrap_or(&0);
                if curr_assigned as u32 <= *num_cores {
                    break;
                }
                let need_reduce = curr_assigned as u32 - *num_cores;
                let mut cleared_cores = 0;
                //clearing.insert(msg.tgid);
                while cleared_cores < need_reduce {
                    //cleared_cores+=1;
                //for j in 0..need_reduce {
                    //if let Some((&clear_core, _)) = core_map.iter().rev().find(|(&y, &x)| x == tgid) {
                    //for (&clear_core, _) in core_map.iter().rev().filter(|(&y, &x)| x == tgid) {
                    for (&clear_core, _) in core_map.iter().filter(|(&y, &x)| x == tgid) {
                        //println!("found {} to remove", clear_core);
                        let proc_pids = self.proc_pids.as_ref().unwrap().read();
                        if let Some(proc_lock) = proc_pids.get(&tgid) {
                            //println!("got lock");
                            let proc_cores = proc_lock.read();
                            if let Some((pid, core)) = proc_cores.iter().find(|(&pid, &core)| core == clear_core) {
                                //cleared_cores += 1;
                                // There's a process that we need to kick off
                                //println!("setting need timer {}", *core);
                                let mut rev_q = self.rev_q.as_ref().unwrap().write();
                                //cleared_cores += 1;
                                //println!("req_q {:?}", rev_q.keys());
                                //if let Some(send_q) = rev_q.get_mut(&(*pid as i32 + 1)) {
                                if let Some(send_q) = rev_q.get_mut(&(*pid as i32)) {
                                    //println!("sending msg {} size {}", *pid, core::mem::size_of::<RevMessage>());
                                    let rev_msg = RevMessage {
                                        reclaim: true
                                    };
                                    send_q.enqueue(rev_msg);
                                    //let mut assigned = self.assigned.as_ref().unwrap().write();
                                    //assigned.remove(&core);
                                    //let mut need_timer = self.need_timer.as_ref().unwrap().write();
                                    //need_timer.insert(*core);
                                    let mut evicting = self.evicting.as_ref().unwrap().write();
                                    evicting.insert(*core, (*pid, 2));
                                    //core_map.insert(clear_core, 0);
                                    cleared_cores += 1;
                                    *clearing = true;
                                    break;
                                //} else {
                                //    //println!("core {} didnt have a q\n", core);
                                }
                                //let mut need_clear = self.need_clear.as_ref().unwrap().write();
                                //need_clear.insert(pid, 2);
                            } else {
                                // no process but core is marked busy.
                                //println!("proc cores {:?}", proc_cores);
                                cleared_cores += 1;
                            }
                        }
                        if cleared_cores == need_reduce {
                            break;
                        }
                    }
                }
            }

            // Distribute free cores to procs that should receive more
            for (&tgid, &msg) in core_requests.iter() {
                let curr_assigned = core_map.values().filter(|&x| *x == tgid).count();
                let num_cores = tgid_num_cores.get(&tgid).unwrap_or(&0);
                if curr_assigned as u32 >= *num_cores {
                    break;
                }
                let need_increase = *num_cores - curr_assigned as u32;
                for j in 0..need_increase {
                    for k in 1..6 {
                        if (core_map.get(&k).is_none() || core_map.get(&k).unwrap() == &0) {
                            core_map.insert(k, tgid);
                            break;
                        }
                    }
                }
            }
            
            for j in 1..6 {
                if !core_map.contains_key(&j) {
                    core_map.insert(j, 0);
                }
            }
            //println!("core map {:?}", core_map);

            // Don't assign core 0
            //for j in 0..msg.prio0 {
            //    core_map.insert(j+1, msg.tgid);
            //}
            //for j in msg.prio0..7 {
            //    if let Some(assigned) = core_map.get(&(j+1)) && *assigned != 0 {
            //        println!("core was assigned");
            //        let proc_pids = self.proc_pids.as_ref().unwrap().read();
            //        if let Some(proc_lock) = proc_pids.get(assigned) {
            //            println!("got lock");
            //            let proc_cores = proc_lock.read();
            //            if let Some((pid, core)) = proc_cores.iter().find(|(&pid, &core)| core == j + 1) {
            //                // There's a process that we need to kick off
            //                // TODO: send message
            //                println!("setting need timer {}", *core);
            //                let mut rev_q = self.rev_q.as_ref().unwrap().write();
            //                if let Some(send_q) = rev_q.get_mut(&(*pid as i32)) {
            //                    let rev_msg = RevMessage {
            //                        reclaim: true
            //                    };
            //                    send_q.enqueue(rev_msg);
            //                }

            //                let mut need_timer = self.need_timer.as_ref().unwrap().write();
            //                need_timer.insert(*core);
            //                let mut evicting = self.evicting.as_ref().unwrap().write();
            //                evicting.insert(*core, *pid);
            //            }
            //        }
            //    }
            //    core_map.insert(j+1, 0);
            //}

        }
    }

    fn unregister_queue(&self, id: i32) -> RingBuffer<UserMessage> {
        let mut user_q = self.user_q.as_ref().unwrap().write();
        let q = user_q.remove(&id);
        println!("freeing queue");
        // We must have a q or this won't be called
        q.unwrap()
    }

    fn unregister_rev_queue(&self, id: i32) -> RingBuffer<RevMessage> {
        let mut rev_q = self.rev_q.as_ref().unwrap().write();
        let q = rev_q.remove(&id);
        println!("freeing queue");
        // We must have a q or this won't be called
        q.unwrap()
    }

    fn task_tick(&self, cpu: i32, queued: bool, guard: RQLockGuard) {
        if cpu == 2 {
            //println!("in periodic tick");
        }
        //let mut need_timer = self.need_timer.as_ref().unwrap().write();
        //if need_timer.contains(&(cpu as u32)) {
        //    println!("starting timer");
        //    hrtick::hrtick_start(cpu, 10000000);
        //    need_timer.remove(&(cpu as u32));
        //}
        if queued {
            let mut clearing = self.clearing.as_ref().unwrap().write();
            let mut assigned = self.assigned.as_ref().unwrap().write();
            let mut must_balance = self.must_balance.as_ref().unwrap().write();
            //let pid_opt = assigned.get(&(cpu as u32));
            //if pid_opt.is_none() {
            //    break;
            //}
            //let pid = pid_opt.unwrap();
            //let mut need_clear = self.need_clear.as_ref().unwrap().write();
            //let clear_opt = need_clear.get_mut(pid);
            //if let Some(clear_val) = clear_opt {
            //    if clear_val == 2 {
            //        *clear_val = 1;
            //        break;
            //    }
            //    if clear_val == 1 {
            //        // Found a task to kick off the cpu
            //        need_clear.clear();
            //        let mut assigned = self.assigned.as_ref().unwrap().write();
            //        must_balance.insert(*pid);
            //        if let Some(ass_pid) = assigned.get(&(cpu as u32)) && ass_pid == pid {
            //            assigned.remove(&(cpu as u32));
            //        }
            //    }
            //}
            let mut core_map = self.core_map.as_ref().unwrap().write();
            let mut evicting = self.evicting.as_ref().unwrap().write();
            let mut should_remove = false;
            if let Some((pid, count)) = evicting.get_mut(&(cpu as u32)) {
                if *count == 2 {
                    hrtick::hrtick_start(cpu, 10000000);
                    *count = 1;
                }
                if *count == 1 {
                    println!("updating in tick\n");
                    must_balance.insert(*pid);
                    if let Some(ass_pid) = assigned.get(&(cpu as u32)) && ass_pid == pid {
                        assigned.remove(&(cpu as u32));
                    }
                    core_map.insert(cpu as u32, 0);
                    *clearing = false;
                }
            }
            if should_remove {
                evicting.remove(&(cpu as u32));
            }
            //    must_balance.insert(*pid);
            //    if let Some(ass_pid) = assigned.get(&(cpu as u32)) && ass_pid == pid {
            //        assigned.remove(&(cpu as u32));
            //    }
            //}
            //evicting.remove(&(cpu as u32));
            resched_cpu(cpu, &guard);
        }
    }

    fn task_departed(&self, pid: u64, _cpu_seqnum: u64, cpu: i32,
                     _from_switchto: i8, _was_current: i8, _guard: RQLockGuard) -> Schedulable {
        println!("in task_departed");
        let mut map = self.map2.as_ref().unwrap().write();
        let sched_lock = map.remove(&pid).unwrap();
        let mut sched_val = sched_lock.write();
        let retval = sched_val.take().unwrap();
        let mut pid_state = self.pid_state.as_ref().unwrap().write();
        let state = pid_state.remove(&pid).unwrap();
        
        let mut proc_pids = self.proc_pids.as_ref().unwrap().write();
        let mut should_remove = false;
        if let Some(proc_lock) = proc_pids.get(&state.tgid) {
            let mut proc_cores = proc_lock.write();
            proc_cores.remove(&pid);
            if proc_cores.is_empty() {
                should_remove = true;
            }
        }
        if should_remove {
            proc_pids.remove(&state.tgid);
        }
        let mut assigned = self.assigned.as_ref().unwrap().write();
        if let Some(assigned_pid) = assigned.get(&(cpu as u32)) && *assigned_pid == pid {
            assigned.remove(&(cpu as u32));
        }
        retval
    }

    fn task_dead(&self, pid: u64, _guard: RQLockGuard) {
        println!("in task_dead");
        let mut map = self.map2.as_ref().unwrap().write();
        //let sched_lock = map.remove(&pid).unwrap();
        map.remove(&pid);
        //let mut sched_val = sched_lock.write();
        //sched_val.take();
        let mut pid_state = self.pid_state.as_ref().unwrap().write();
        let state = pid_state.remove(&pid).unwrap();
        let qs = self.qs.as_ref().unwrap().read();
        let mut old_q = qs.get(&0).unwrap().write();
        //let mut new_q = qs.get(&cpu).unwrap().write();
        if let Some(idx) = old_q.iter().position(|&x| x == pid) {
            old_q.remove(idx);
        }
        
        let mut should_remove = false;
        let mut proc_pids = self.proc_pids.as_ref().unwrap().write();
        if let Some(proc_lock) = proc_pids.get(&state.tgid) {
            let mut proc_cores = proc_lock.write();
            proc_cores.remove(&pid);
            if proc_cores.is_empty() {
                should_remove = true;
            }
        }
        if should_remove {
            proc_pids.remove(&state.tgid);
        }
        let mut assigned = self.assigned.as_ref().unwrap().write();
        let found_assigned = assigned.iter().find(|(&x, &y)| y == pid);
        if let Some((assigned_cpu, _)) = found_assigned {
            let cpu = *assigned_cpu;
            assigned.remove(&cpu);
        }
        println!("task_dead done");
    }

}

//bento::kernel_module!(
//    BentoGhostModule,
//    author: b"Bento Contributors",
//    description: b"kernel module to replace the scheduler",
//    license: b"GPL"
//);
