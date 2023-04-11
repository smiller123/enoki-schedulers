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

use core::mem;
use core::str;
use core::fmt::Debug;

use self::ringbuffer::RingBuffer;
use self::sched_core::resched_cpu;

#[repr(C)]
pub struct BentoSched {
    pub q: Option<RwLock<VecDeque<u64>>>,
    pub map: Option<RwLock<BTreeMap<u64, Schedulable>>>,
    pub moved: Option<RwLock<BTreeSet<u64>>>,
    pub user_q: Option<RwLock<BTreeMap<i32, RingBuffer<UserMessage>>>>,
    pub rev_q: Option<RwLock<BTreeMap<i32, RingBuffer<UserMessage>>>>,
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

    //fn init(&mut self) {
   // }

    fn task_new(&self, pid: u64, _runtime: u64, runnable: u16, _prio: i32, sched: Schedulable, _guard: RQLockGuard) {
        //println!("In task_new");
        if runnable > 0 {
            let mut q = self.q.as_ref().unwrap().write();
            q.push_back(pid);
            let mut map = self.map.as_ref().unwrap().write();
            if map.get(&pid).is_none() {
                map.insert(pid, sched);
            }
        }
    }

    fn task_wakeup(&self, pid: u64, _agent_data: u64, deferrable: bool,
                   _last_run_cpu: i32, _wake_up_cpu: i32, _waker_cpu: i32,
                   sched: Schedulable, _guard: RQLockGuard) {
        //println!("In task_wakeup");
        let mut q = self.q.as_ref().unwrap().write();
        let mut map = self.map.as_ref().unwrap().write();
        if deferrable {
            q.push_back(pid);
        } else {
            q.push_front(pid);
        }
        //map.insert(pid, wake_up_cpu as u32);
        map.insert(pid, sched);
    }

    fn task_preempt(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64, _cpu: i32,
                    _from_switchto: i8, _was_latched: i8, sched: Schedulable, _guard: RQLockGuard) {
        //println!("In task_preempt");
        let mut q = self.q.as_ref().unwrap().write();
        q.push_back(pid);
        let mut map = self.map.as_ref().unwrap().write();
        map.insert(pid, sched);
    }

    fn task_blocked(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64,
                    _cpu: i32, _from_switchto: i8, _guard: RQLockGuard) {
        //println!("In task_blocked");
        let mut q = self.q.as_ref().unwrap().write();
        // position depends on iterator state, so don't use iterator before we call position
        if q.contains(&pid) {
            if let Some(idx) = q.iter().position(|&x| x == pid) {
                q.remove(idx);
            }
        }
        // let mut map = self.map.as_ref().unwrap().write();
        //map.insert(pid, cpu as u32);
        // map.insert(pid, sched);
    }

    fn pick_next_task(&self, cpu: i32, curr_sched: Option<Schedulable>, 
                      curr_runtime: Option<u64>, _guard: RQLockGuard) -> Option<Schedulable> {
        let mut q = self.q.as_ref().unwrap().write();
        let mut map = self.map.as_ref().unwrap().write();
       
        //println!("In pick next task");

        let mut idx : usize = 0usize;
        while idx < q.len() {
            // cycle through the queue until we find first task belonging to this CPU
            if let Some(pid) = q.get(idx) {
                if map.get(&pid).is_some() {
                    // if this task's cpu matches current target cpu, use this task
                    if (map.get(&pid).unwrap().get_cpu() == cpu as u32 ||
                        map.get(&pid).unwrap().get_cpu() == u32::MAX) {
                        let pid = q.remove(idx).unwrap();
                        let mut moved = self.moved.as_ref().unwrap().write();
                        moved.remove(&pid);
                        hrtick::hrtick_start(cpu, 10000);
                        return map.remove(&pid);
                        //break;
                    }
                }
            }
            idx += 1;
        }

        //if (idx >= q.len()) {
        //    //println!("Returning none 1 from pick_next_task");
        //    return None;
        //}

        //if let Some(pid) = q.remove(idx) {
        //    hrtick::hrtick_start(cpu, 10000);

        //    //println!("Returning Some from pick_next_task");
        //    //println!("pid = {}, cpu = {}", map.get(&pid).unwrap().get_pid(), map.get(&pid).unwrap().get_cpu());
        //    return map.remove(&pid);
        //}

        //println!("Returning none 2 from pick_next_task");

        return None;

        // if let Some(pid) = q.pop_front() {
            // let map = self.map.as_ref().unwrap().read();
            // if map.get(&pid).is_none() {
                // println!("Terrible problem, pid has no cpu");
                // q.push_front(pid);
                // return None;
            // }
            // if (map.get(&pid).unwrap().get_cpu() == cpu as u32 ||
                // map.get(&pid).unwrap().get_cpu() == u32::MAX) {

                //println!("pnt cpu {} would pick {}", cpu, pid);
                //return None;
                //Some(pid)
                // None shouldn't happen anymore
                // TODO: probably should turn this into a constant
                // hrtick::hrtick_start(cpu, 10000);
                // Some(*map.get(&pid).unwrap())
            // } else {
                //println!("can't schedule on other cpu\n");
                // q.push_front(pid);
                // None
            // }
        //} else {
            //None
        //}
    }

    fn pnt_err(&self, cpu: i32, pid: u64, err: i32, _sched: Option<Schedulable>, _guard: RQLockGuard) {
        //println!("In pnt_err");
        //let mut map = self.map.as_ref().unwrap().write();
        //map.insert(sched.get_pid(), sched);
    }

    fn select_task_rq(&self, pid: u64, _waker_cpu: i32, _prev_cpu: i32) -> i32 {
        //println!("In select_task_rq");
        let mut map = self.map.as_ref().unwrap().write();
        //*map.get(&pid).unwrap_or(&1) as i32
        match map.get(&pid) {
            //None => {
            //    //map.insert(pid, pid as u32 % 2);
            //    pid as i32 % 2
            //},
            None => 1,
            Some(sched) => sched.get_cpu() as i32,
        }
    }

    fn selected_task_rq(&self, sched: Schedulable) {
        //println!("In selected_task_rq");
        let mut map = self.map.as_ref().unwrap().write();
        map.insert(sched.get_pid(), sched);
    }

    //fn migrate_task_rq(&self, pid: u64, new_cpu: i32) {
    fn migrate_task_rq(&self, pid: u64, sched: Schedulable, _guard: RQLockGuard) -> Schedulable {
        //println!("in migrate");
        //println!("hitting migrate_task_rq pid {} to cpu {}", pid, new_cpu);
        let mut map = self.map.as_ref().unwrap().write();
        //map.insert(pid, new_cpu as u32);
        //
        let old_sched = map.remove(&pid).unwrap();

        map.insert(pid, sched);

        // TODO: sus?
        return old_sched;
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
        //println!("In balance");
        let q = self.q.as_ref().unwrap().read();
        let mut next = &0;
        if let Some(opt) = q.front() {
            next = opt;
            let mut map = self.map.as_ref().unwrap().read();
            if let Some(sched) = map.get(opt) && sched.get_cpu() == cpu as u32 {
                return None;
            }
            let mut moved = self.moved.as_ref().unwrap().write();
            if moved.contains(next) {
                return None;
            }
            //println!("would move {} to cpu {}", *next, cpu);
            moved.insert(*next);
            //return None;
            return Some(*next);
        } else {
            return None;
        }
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
        // return None;
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
        //println!("q ptr {:?}", (*q.inner).offset);
        //println!("q capacity {}", (*q.inner).capacity);
        //println!("q readptr {}", (*q.inner).readptr);
        //println!("q writeptr {}", (*q.inner).writeptr);
        let mut user_q = self.user_q.as_ref().unwrap().write();
        let next = user_q.keys().max().map_or(0, |max| max + 1);
        user_q.insert(next as i32, q);

        return next as i32;
        // user_q.replace(q);
        //

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
        // rev_q.replace(q);

        //self.user_q = Some(RwLock::new(q));
        }
    }

    fn enter_queue(&self, id: i32, entries: u32) {
        let mut user_q_list = self.user_q.as_ref().unwrap().write();
        let user_q = user_q_list.get_mut(&id).unwrap();


        // let mut user_q_guard = self.user_q.as_ref().unwrap().write();
        // let user_q = user_q_guard.as_mut().unwrap();
        for i in 0..entries {
        //unsafe {
        //println!("user_q head {}", (*user_q.inner).writeptr);
        //println!("user_q tail {}", (*user_q.inner).readptr);
        //}
            let msg = user_q.dequeue();
        //println!("msg val {:?}", msg);
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
        if queued {
            //println!("ticking {}", cpu);
            unsafe {
                // TODO: If we are including a guard parameter, shouldn't we use
                // some function that acquires the lock?
                //bento::bindings::resched_cpu_no_lock(cpu);
                resched_cpu(cpu, &guard);
            }
        }
    }

    fn task_departed(&self, pid: u64, _cpu_seqnum: u64, _cpu: i32, 
                     _from_switchto: i8, _was_current: i8, _guard: RQLockGuard) -> Schedulable {
        println!("in task_departed");
        let mut map = self.map.as_ref().unwrap().write();
        map.remove(&pid).unwrap()
    }
}

//bento::kernel_module!(
//    BentoGhostModule,
//    author: b"Bento Contributors",
//    description: b"kernel module to replace the scheduler",
//    license: b"GPL"
//);
