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

use core::mem;
use core::str;

use self::ringbuffer::RingBuffer;

#[repr(C)]
pub struct BentoSched {
    pub q: Option<RwLock<VecDeque<u64>>>,
    pub qs: Option<RwLock<BTreeMap<u32,RwLock<VecDeque<u64>>>>>,
    pub map: Option<RwLock<BTreeMap<u64, Schedulable>>>,
    pub map2: Option<RwLock<BTreeMap<u64, RwLock<Option<Schedulable>>>>>,
    pub user_q: Option<RwLock<BTreeMap<i32, RingBuffer<UserMessage>>>>,
    pub rev_q: Option<RwLock<BTreeMap<i32, RingBuffer<UserMessage>>>>,
    pub pid_to_hint: Option<RwLock<BTreeMap<u64, u32>>>,
    pub hint_to_core: Option<RwLock<BTreeMap<u32, i32>>> 
}

pub struct UpgradeData {
    q: Option<RwLock<VecDeque<u64>>>,
    map: Option<RwLock<BTreeMap<u64, Schedulable>>>
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct UserMessage {
    tid: u32,
    hint: u32
}

impl BentoScheduler<'_, '_, UpgradeData, UpgradeData, UserMessage, UserMessage> for BentoSched {
    fn get_policy(&self) -> i32 {
        10
    }

    fn task_new(&self, pid: u64, _tgid: u64, _runtime: u64, runnable: u16, _prio: i32, 
                sched: Schedulable, _guard: RQLockGuard) {
        if runnable > 0 {
            let qs = self.qs.as_ref().unwrap().read();
            let mut q = qs.get(&sched.get_cpu()).unwrap().write();
            q.push_back(pid);
            let mut map = self.map2.as_ref().unwrap().write();
            if map.get(&pid).is_none() {
                map.insert(pid, RwLock::new(Some(sched)));
            }
        } else {
            let mut map = self.map2.as_ref().unwrap().write();
            if map.get(&pid).is_none() {
                map.insert(pid, RwLock::new(Some(sched)));
            }
        }
    }

    fn task_wakeup(&self, pid: u64, _agent_data: u64, deferrable: bool,
                   _last_run_cpu: i32, _wake_up_cpu: i32, _waker_cpu: i32,
                   sched: Schedulable, _guard: RQLockGuard) {
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();
        let map = self.map2.as_ref().unwrap().read();
        if deferrable {
            q.push_back(pid);
        } else {
            q.push_front(pid);
        }
        let mut sched_val = map.get(&pid).unwrap().write();
        sched_val.replace(sched);
    }

    fn task_preempt(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64, _cpu: i32,
                    _from_switchto: i8, _was_latched: i8, sched: Schedulable, _guard: RQLockGuard) {
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();
        q.push_back(pid);
        let map = self.map2.as_ref().unwrap().read();
        let mut sched_val = map.get(&pid).unwrap().write();
        sched_val.replace(sched);
    }

    fn task_blocked(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64,
                    cpu: i32, _from_switchto: i8, _guard: RQLockGuard) {
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&(cpu as u32)).unwrap().write();
        // position depends on iterator state, so don't use iterator before we call position
        if let Some(idx) = q.iter().position(|&x| x == pid) {
            q.remove(idx);
        }
    }

    fn task_yield(
        &self, pid: u64, _runtime: u64,
        _cpu_seqnum: u64, _cpu: i32, _from_switchto: i8, sched: Schedulable, _guard: RQLockGuard
    ) {
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();
        let map = self.map2.as_ref().unwrap().read();
        let sched_lock = map.get(&pid).unwrap();
        let mut sched_val = sched_lock.write();
        if !q.contains(&pid) {
            q.push_back(pid);
        }
        sched_val.replace(sched);
    }

    fn pick_next_task(&self, cpu: i32, _curr_sched: Option<Schedulable>, _curr_runtime: Option<u64>, _guard: RQLockGuard) -> Option<Schedulable> {
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&(cpu as u32)).unwrap().write();

        let mut idx : usize = 0usize;
        while idx < q.len() {
            // TODO: change this to write() or read() ?
            let pid = q.get(idx).unwrap();
            let map = self.map2.as_ref().unwrap().read();
            let mut sched_val = map.get(&pid).unwrap().write();
            if sched_val.is_none() {
                println!("Terrible problem, pid has no cpu");
                return None;
            }
            if sched_val.as_ref().unwrap().get_cpu() == cpu as u32 ||
                sched_val.as_ref().unwrap().get_cpu() == u32::MAX {

                let sched = sched_val.take();
                q.remove(idx);
                return sched;
            }
            idx += 1;
        }
        None
    }

    fn select_task_rq(&self, pid: u64, _waker_cpu: i32, _prev_cpu: i32) -> i32 {
        let map = self.map2.as_ref().unwrap().read();
        let pid_hint_map = self.pid_to_hint.as_ref().unwrap().read();
        let hint_core_map = self.hint_to_core.as_ref().unwrap().read();

        // prioritize matching hints before currently assigned core
        let reval = match pid_hint_map.get(&pid) {
            None => match map.get(&pid) {
                        None => pid as i32 % 6,
                        Some(sched_lock) => {
                            let sched_opt = sched_lock.read();
                            if sched_opt.is_some() {
                                let sched = sched_opt.as_ref().unwrap();
                                sched.get_cpu() as i32
                            } else {
                                pid as i32 % 6
                            }
                        }
                    },
            Some(hint) => match hint_core_map.get(&hint) {
                            None => 1, // something has gone really wrong...
                            Some(core) => *core
                        },
        };
        return reval;
    }

    fn migrate_task_rq(&self, pid: u64, sched: Schedulable, _guard: RQLockGuard) -> Schedulable {
        let qs = self.qs.as_ref().unwrap().read();
        let map = self.map2.as_ref().unwrap().read();
        let mut sched_val = map.get(&pid).unwrap().write();
        let cpu = sched.get_cpu();
        if sched_val.is_some() {
            let old_sched = sched_val.replace(sched);
            let mut old_q = qs.get(&old_sched.as_ref().unwrap().get_cpu()).unwrap().write();
            let mut new_q = qs.get(&cpu).unwrap().write();
            if let Some(idx) = old_q.iter().position(|&x| x == pid) {
                old_q.remove(idx);
            }
            new_q.push_back(pid);
            return old_sched.unwrap();
        } else {
            // not runnable anyway, so don't worry, we'll fix it in wakeup or whatever.
            return sched;
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
        println!("start registering new queue");
        let mut user_q = self.user_q.as_ref().unwrap().write();
        let next = user_q.keys().max().map_or(0, |max| max + 1);
        println!("Obtained current user queue");
        user_q.insert(next as i32, q);
        println!("Replacing with passed-in queue");
        return next as i32;
    }

    fn register_reverse_queue(&self, _pid: u64, q: RingBuffer<UserMessage>) -> i32 {
        let mut rev_q = self.rev_q.as_ref().unwrap().write();
        let next = rev_q.keys().max().map_or(0, |max| max + 1);
        rev_q.insert(next as i32, q);
        return next as i32;
    }

    fn enter_queue(&self, id: i32, entries: u32) {
        println!("Entries to poll {}", entries);
        let mut user_q_list = self.user_q.as_ref().unwrap().write();
        let user_q = user_q_list.get_mut(&id).unwrap();
        for _i in 0..entries {
            let msg = user_q.dequeue().unwrap();
            println!("msg val {:?}", &msg.hint);

            let mut hint_map = self.hint_to_core.as_ref().unwrap().write();
            let mut pid_hint_map = self.pid_to_hint.as_ref().unwrap().write();

            // TODO: change this to be slightly less load-intense on certain cores
            // For now, we are assigning a new hint to the mod of the hint core
            hint_map.entry(msg.hint).or_insert_with(|| {
                (((msg.hint) % 6) + 1) as i32
            });

            // unconditionally insert pid and hint, may want to
            // migrate process to hint's core
            pid_hint_map.insert(msg.tid as u64, msg.hint); 
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

    fn task_departed(&self, pid: u64, _cpu_seqnum: u64, _cpu: i32,
                     _from_switchto: i8, _was_current: i8, _guard: RQLockGuard) -> Schedulable {
        println!("in task_departed");
        let mut map = self.map2.as_ref().unwrap().write();
        let sched_lock = map.remove(&pid).unwrap();
        let mut sched_val = sched_lock.write();
        sched_val.take().unwrap()
    }

    fn task_dead(&self, pid: u64, _guard: RQLockGuard) {
        println!("in task_dead");
        let mut map = self.map2.as_ref().unwrap().write();
        let sched_lock = map.remove(&pid).unwrap();
        let mut sched_val = sched_lock.write();
        sched_val.take().unwrap();
    }

}
