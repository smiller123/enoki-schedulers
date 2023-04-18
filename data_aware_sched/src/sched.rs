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

    //fn init(&mut self) {
   // }

    fn task_new(&self, pid: u64, _runtime: u64, runnable: u16, _prio: i32, 
                sched: Schedulable, _guard: RQLockGuard) {
        //println!("tid in task_new: {}", pid);
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
        //let mut q = self.q.as_ref().unwrap().write();
        let mut map = self.map2.as_ref().unwrap().read();
        if deferrable {
            q.push_back(pid);
        } else {
            q.push_front(pid);
        }
        //map.insert(pid, wake_up_cpu as u32);
        let mut sched_val = map.get(&pid).unwrap().write();
        sched_val.replace(sched);
        //map.insert(pid, sched);
    }

    fn task_preempt(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64, _cpu: i32,
                    _from_switchto: i8, _was_latched: i8, sched: Schedulable, _guard: RQLockGuard) {
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();
        //let mut q = self.q.as_ref().unwrap().write();
        q.push_back(pid);
        let mut map = self.map2.as_ref().unwrap().read();
        let mut sched_val = map.get(&pid).unwrap().write();
        sched_val.replace(sched);
        //map.insert(pid, sched);
    }

    fn task_blocked(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64,
                    cpu: i32, _from_switchto: i8, _guard: RQLockGuard) {
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&(cpu as u32)).unwrap().write();
        //let mut q = self.q.as_ref().unwrap().write();
        // position depends on iterator state, so don't use iterator before we call position
        if let Some(idx) = q.iter().position(|&x| x == pid) {
            q.remove(idx);
        }
        //let mut map = self.map.as_ref().unwrap().write();
        //map.insert(pid, cpu as u32);
        //map.insert(pid, sched);
    }

    fn task_yield(
        &self, pid: u64, runtime: u64,
        _cpu_seqnum: u64, cpu: i32, _from_switchto: i8, sched: Schedulable, _guard: RQLockGuard
    ) {
        //let mut map = self.map.as_ref().unwrap().write();
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&sched.get_cpu()).unwrap().write();
        let map = self.map2.as_ref().unwrap().read();
        let sched_lock = map.get(&pid).unwrap();
        let mut sched_val = sched_lock.write();
        //let mut q = self.q.as_ref().unwrap().write();
        //let mut pid_state = self.pid_state.as_ref().unwrap().write();
        if !q.contains(&pid) {
            q.push_back(pid);
        }
        //pid_state.insert(pid, sched.get_cpu());
        sched_val.replace(sched);
    }

    fn pick_next_task(&self, cpu: i32, curr_sched: Option<Schedulable>, curr_runtime: Option<u64>, _guard: RQLockGuard) -> Option<Schedulable> {
        let qs = self.qs.as_ref().unwrap().read();
        let mut q = qs.get(&(cpu as u32)).unwrap().write();
        //let mut q = self.q.as_ref().unwrap().write();
        // let map = self.map.as_ref().unwrap().read();
        
        // let mut idx : usize = 0usize;
        // while idx < q.len() {
            // // cycle through the queue until we find first task belonging to this CPU
            // if let Some(pid) = q.get(idx) {
                // if map.get(&pid).is_some() {
                    // // if this task's cpu matches current target cpu, use this task
                    // if (map.get(&pid).unwrap().get_cpu() == cpu as u32 ||
                        // map.get(&pid).unwrap().get_cpu() == u32::MAX) {
                        // break;
                    // }
                // }
            // }
            // idx += 1;
        // }

        // if (idx >= q.len()) {
            // return None;
        // }

        // if let Some(pid) = q.remove(idx) {
            // hrtick::hrtick_start(cpu, 10000);
            // return Some(*map.get(&pid).unwrap());
        // }

        // return None;

        let mut idx : usize = 0usize;
        while idx < q.len() {
        //if let Some(pid) = q.pop_front() {
            // TODO: change this to write() or read() ?
            let pid = q.get(idx).unwrap();
            let mut map = self.map2.as_ref().unwrap().read();
            let mut sched_val = map.get(&pid).unwrap().write();
            if sched_val.is_none() {
                println!("Terrible problem, pid has no cpu");
                //q.push_front(pid);
                return None;
            }
            if (sched_val.as_ref().unwrap().get_cpu() == cpu as u32 ||
                sched_val.as_ref().unwrap().get_cpu() == u32::MAX) {

                //println!("pnt cpu {} would pick {}", cpu, pid);
                //return None;
                //Some(pid)
                // None shouldn't happen anymore
                // TODO: probably should turn this into a constant
                // hrtick::hrtick_start(cpu, 10000);
                //Some(*map.get(&pid).unwrap())
                // TODO: I hope this works...
                //let sched = map.remove(&pid);
                let sched = sched_val.take();
                q.remove(idx);
                return sched;
            }
                //println!("can't schedule on other cpu\n");
                //q.push_front(pid);
            //    None
            //}
            idx += 1;
        }
        None
    }

    fn pnt_err(&self, cpu: i32, pid: u64, err: i32, _sched: Option<Schedulable>, _guard: RQLockGuard) {
        // TODO: I think we can just comment this out in pnt_err?
        //let mut map = self.map.as_ref().unwrap().write();
        //map.insert(sched.get_pid(), sched);
    }

    fn select_task_rq(&self, pid: u64, _waker_cpu: i32, _prev_cpu: i32) -> i32 {
        //println!("tid in select_task_rq: {}", pid);
        let map = self.map2.as_ref().unwrap().read();
        //*map.get(&pid).unwrap_or(&1) as i32
        //
        //
        let pid_hint_map = self.pid_to_hint.as_ref().unwrap().read();
        let hint_core_map = self.hint_to_core.as_ref().unwrap().read();

        // prioritize matching hints before currently assigned core
        let reval = match pid_hint_map.get(&pid) {
            None => (match map.get(&pid) {
                        None => pid as i32 % 6,
                        Some(sched_lock) => {
                            let sched_opt = sched_lock.read();
                            if sched_opt.is_some() {
                            //if let Some(sched) = sched_opt {
                                let sched = sched_opt.as_ref().unwrap();
                                sched.get_cpu() as i32
                            } else {
                                pid as i32 % 6
                            }
                        }
                    }),
            Some(hint) => (match hint_core_map.get(&hint) {
                            None => 1, // something has gone really wrong...
                            Some(core) => *core
                        }),
        };
            return reval;
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
        //println!("hitting migrate_task_rq pid {} to cpu {}", pid, new_cpu);
        let qs = self.qs.as_ref().unwrap().read();
        let mut map = self.map2.as_ref().unwrap().read();
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
        //map.insert(pid, new_cpu as u32);
        //let old_sched = map.remove(&pid).unwrap();
        //map.insert(pid, sched);

    }

    fn balance(&self, cpu: i32, _guard: RQLockGuard) -> Option<u64> {
        // TODO: balance should probably be fine? May need to change
        // pick_next_task so that scheduler does not give up selecting
        // a new process 
        //
        // TODO: make sure 
        //if cpu != 1 {
            //return None;
        //}
        // GOOD BAlANCE CODE FOR SHINJUKU
        // let q = self.q.as_ref().unwrap().read();
        // let mut next = &0;
        // if let Some(opt) = q.front() {
            // next = opt;
            // return Some(*next);
        // } else {
            // return None;
        // }
        // let mut map = self.map.as_ref().unwrap().write();
        // if let Some(old_sched) = map.get(next) {
            // let new_sched = Schedulable {
            //    pid: *next,
            //    cpu: cpu as u32
            // };
            // println!("shifting proc {} to cpu 1", *next);
            // map.insert(*next, new_sched);
            // return Some(*next);
        // }
        return None;
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

    fn register_queue(&self, q: RingBuffer<UserMessage>) -> i32 {
        //println!("q ptr {:?}", q.inner);
        unsafe {
        println!("start registering new queue");
        //println!("q ptr {:?}", (*q.inner).offset);
        //println!("q capacity {}", (*q.inner).capacity);
        //println!("q readptr {}", (*q.inner).readptr);
        //println!("q writeptr {}", (*q.inner).writeptr);
        let mut user_q = self.user_q.as_ref().unwrap().write();
        let next = user_q.keys().max().map_or(0, |max| max + 1);
        println!("Obtained current user queue");
        user_q.insert(next as i32, q);
        println!("Replacing with passed-in queue");
        return next as i32;

        //self.user_q = Some(RwLock::new(q));
        }
    }

    fn register_reverse_queue(&self, q: RingBuffer<UserMessage>) -> i32 {
        //println!("q ptr {:?}", q.inner);
        unsafe {
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
    }

    fn enter_queue(&self, id: i32, entries: u32) {
        println!("Entries to poll {}", entries);
        let mut user_q_list = self.user_q.as_ref().unwrap().write();
        let user_q = user_q_list.get_mut(&id).unwrap();
        for i in 0..entries {
        //unsafe {
        //println!("user_q head {}", (*user_q.inner).writeptr);
        //println!("user_q tail {}", (*user_q.inner).readptr);
        //}
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

    // fn task_tick(&self, cpu: i32, queued: bool) {
        // if queued {
            // //println!("ticking {}", cpu);
            // unsafe {
                // bento::bindings::resched_cpu_no_lock(cpu);
            // }
        // }
    // 
    // }
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

//bento::kernel_module!(
//    BentoGhostModule,
//    author: b"Bento Contributors",
//    description: b"kernel module to replace the scheduler",
//    license: b"GPL"
//);
