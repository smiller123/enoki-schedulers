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

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;
use alloc::collections::btree_set::BTreeSet;

use core::mem;
use core::str;

use self::ringbuffer::RingBuffer;
use self::sched_core::resched_cpu;
use bento::kernel::time::Timespec64;
use bento::kernel::time::getnstimeofday64_rs;

#[repr(C)]
pub struct BentoSched {
    pub qs: Option<RwLock<BTreeMap<u32, RwLock<VecDeque<u64>>>>>,
    pub q: Option<RwLock<VecDeque<u64>>>,
    pub map2: Option<RwLock<BTreeMap<u64, RwLock<Option<Schedulable>>>>>,
    pub map: Option<RwLock<BTreeMap<u64, Schedulable>>>,
    pub moved: Option<RwLock<BTreeSet<u64>>>,
    pub pid_state: Option<RwLock<BTreeMap<u64, u32>>>,
    pub cpu_running: Option<RwLock<BTreeSet<u32>>>,
    pub user_q: Option<RwLock<BTreeMap<i32, RingBuffer<UserMessage>>>>,
    pub rev_q: Option<RwLock<BTreeMap<i32, RingBuffer<UserMessage>>>>,
    pub timing: Option<RwLock<BTreeMap<u64, Timespec64>>>,
}

pub struct UpgradeData {
    q: Option<RwLock<VecDeque<u64>>>,
    map: Option<RwLock<BTreeMap<u64, Schedulable>>>
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct UserMessage {
    val: u32
}

impl BentoScheduler<'_, '_, UpgradeData, UpgradeData, UserMessage, UserMessage> for BentoSched {
    fn get_policy(&self) -> i32 {
        10
    }

    fn task_new(&self, pid: u64, _tgid: u64, _runtime: u64, runnable: u16, _prio: i32, sched: Schedulable, _guard: RQLockGuard) {
        if runnable > 0 {
            let mut map = self.map2.as_ref().unwrap().write();
            let qs = self.qs.as_ref().unwrap().read();
            let mut q = qs.get(&sched.get_cpu()).unwrap().write();
            let mut pid_state = self.pid_state.as_ref().unwrap().write();
            let cpu = sched.get_cpu();
            if !q.contains(&pid) {
                q.push_back(pid);
            }
            map.insert(pid, RwLock::new(Some(sched)));
            if cpu != u32::MAX {
                pid_state.insert(pid, cpu);
            }
        } else {
            let mut map = self.map2.as_ref().unwrap().write();
            map.insert(pid, RwLock::new(Some(sched)));
        }
    }

    fn task_wakeup(&self, pid: u64, _agent_data: u64, _deferrable: bool,
                   _last_run_cpu: i32, _wake_up_cpu: i32, _waker_cpu: i32,
                   sched: Schedulable, guard: RQLockGuard) {
        let map = self.map2.as_ref().unwrap().read();
        let sched_lock = map.get(&pid).unwrap();
        let mut sched_val = sched_lock.write();
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();
        if !q.contains(&pid) {
            q.push_back(pid);
        }
        let cpu = sched.get_cpu();
        sched_val.replace(sched);
        if q.len() == 1 {
            resched_cpu(cpu as i32, &guard);
        }
    }

    fn task_preempt(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64, _cpu: i32,
                    _from_switchto: i8, _was_latched: i8, sched: Schedulable, _guard: RQLockGuard) {
        let map = self.map2.as_ref().unwrap().read();
        let sched_lock = map.get(&pid).unwrap();
        let mut sched_val = sched_lock.write();

        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();

        if !q.contains(&pid) {
            q.push_back(pid);
        }
        sched_val.replace(sched);
    }

    fn task_blocked(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64,
                    cpu: i32, _from_switchto: i8, _guard: RQLockGuard) {
        let map = self.map2.as_ref().unwrap().read();
        let sched_lock = map.get(&pid).unwrap();
        let mut sched_val = sched_lock.write();
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&(cpu as u32)).unwrap().write();
        // position depends on iterator state, so don't use iterator before we call position
        if q.contains(&pid) {
            if let Some(idx) = q.iter().position(|&x| x == pid) {
                q.remove(idx);
            }
        }
        sched_val.take();
        {
        let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
        cpu_running.remove(&(cpu as u32));
        }
    }

    fn task_yield(
        &self, pid: u64, _runtime: u64,
        _cpu_seqnum: u64, cpu: i32, _from_switchto: i8, sched: Schedulable, _guard: RQLockGuard
    ) {
        let map = self.map2.as_ref().unwrap().read();
        let sched_lock = map.get(&pid).unwrap();
        let mut sched_val = sched_lock.write();
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();
        if !q.contains(&pid) {
            q.push_back(pid);
        }
        sched_val.replace(sched);
        {
        let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
        cpu_running.remove(&(cpu as u32));
        }
    }

    fn pick_next_task(&self, cpu: i32, _curr_sched: Option<Schedulable>, 
                      _curr_runtime: Option<u64>, _guard: RQLockGuard) -> Option<Schedulable> {
        let mut start = Timespec64::new();
        getnstimeofday64_rs(&mut start);
        let map = self.map2.as_ref().unwrap().read();
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&(cpu as u32)).unwrap().write();

        if let Some(pid) = q.pop_front() {
            let sched_lock = map.get(&pid).unwrap();
            let mut sched_val = sched_lock.write();
            if sched_val.is_some() {
                let retval = sched_val.take().unwrap();
                if retval.get_cpu() == cpu as u32 {
                    let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
                    cpu_running.insert(cpu as u32);
                    hrtick::hrtick_start(cpu, 10000);
                    return Some(retval);
                } else {
                    // Something has gone horribly wrong
                    sched_val.replace(retval);
                    q.push_back(pid);
                }
            }
        }
        let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
        cpu_running.remove(&(cpu as u32));

        return None;
    }

    fn pnt_err(&self, cpu: i32, _pid: u64, _err: i32, _sched: Option<Schedulable>, _guard: RQLockGuard) {
        let mut cpu_running = self.cpu_running.as_ref().unwrap().write();
        cpu_running.remove(&(cpu as u32));
    }

    fn select_task_rq(&self, pid: u64, _waker_cpu: i32, _prev_cpu: i32) -> i32 {
        let pid_state = self.pid_state.as_ref().unwrap().read();
        match pid_state.get(&pid) {
            None => {
                (pid as i32 % 5) + 3
            },
            Some(cpu) => *cpu as i32,
        }
    }

    fn selected_task_rq(&self, sched: Schedulable) {
        let mut pid_state = self.pid_state.as_ref().unwrap().write();
        pid_state.insert(sched.get_pid(), sched.get_cpu());
    }

    fn migrate_task_rq(&self, pid: u64, sched: Schedulable, _guard: RQLockGuard) -> Schedulable {
        let map = self.map2.as_ref().unwrap().read();
        let sched_lock = map.get(&pid).unwrap();
        let mut sched_val = sched_lock.write();

        let qs = self.qs.as_ref().unwrap().read();
        let mut new_q = qs.get(&sched.get_cpu()).unwrap().write();
        let mut pid_state = self.pid_state.as_ref().unwrap().write();

        let cpu = sched.get_cpu();
        let old_sched = sched_val.replace(sched).unwrap();
        let mut old_q = qs.get(&old_sched.get_cpu()).unwrap().write();
        if let Some(idx) = old_q.iter().position(|&x| x == pid) {
            old_q.remove(idx);
        }
        if !new_q.contains(&pid) {
            new_q.push_front(pid);
        }

        pid_state.insert(pid, cpu);

        // TODO: sus?
        let mut moved = self.moved.as_ref().unwrap().write();
        moved.remove(&pid);
        return old_sched;
    }

    fn balance(&self, cpu: i32, _guard: RQLockGuard) -> Option<u64> {
        if cpu < 3 {
            return None;
        }
        let qs = self.qs.as_ref().unwrap().read();
        let q_len = {
            let q = qs.get(&(cpu as u32)).unwrap().read();
            q.len()
        };
        let mut longest_q = None;
        let mut max_q_len = q_len;
        for i in 0..8 {
            if i == cpu {
                continue;
            }
            let q = qs.get(&(i as u32)).unwrap().read();
            if q.len() > max_q_len {
                max_q_len = q.len();
                longest_q = Some(i);
            }

        }
        if longest_q.is_none() {
            return None;
        }
        let q = qs.get(&(longest_q.unwrap() as u32)).unwrap().read();
        let mut moved = self.moved.as_ref().unwrap().write();
        let next;
        if let Some(opt) = q.front() {
            next = opt;
            if moved.contains(next) {
                return None;
            }
            moved.insert(*next);
            return Some(*next);
        } else {
            return None;
        }
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
        if queued {
            resched_cpu(cpu, &guard);
        }
    }

    fn task_departed(&self, pid: u64, _cpu_seqnum: u64, cpu: i32, 
                     _from_switchto: i8, _was_current: i8, _guard: RQLockGuard) -> Schedulable {
        let mut map = self.map2.as_ref().unwrap().write();
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&(cpu as u32)).unwrap().write();
        if let Some(idx) = q.iter().position(|&x| x == pid) {
            q.remove(idx);
        }
        map.remove(&pid).unwrap().write().take().unwrap()
    }

    fn task_dead(&self, pid: u64, _guard: RQLockGuard) {
        let mut map = self.map2.as_ref().unwrap().write();
        if let Some(sched_lock) = map.remove(&pid) {
            let sched_opt = sched_lock.write().take();
            if let Some(sched) = sched_opt {
                let qs = self.qs.as_ref().unwrap().read();
                let mut q = qs.get(&sched.get_cpu()).unwrap().write();
                if let Some(idx) = q.iter().position(|&x| x == pid) {
                    q.remove(idx);
                }
            }
        }
    }
}
