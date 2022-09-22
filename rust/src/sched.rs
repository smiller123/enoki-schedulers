#[cfg(not(feature = "replay"))]
use bento::println;
#[cfg(not(feature = "replay"))]
use bento::scheduler_utils;
#[cfg(feature = "replay")]
use scheduler_utils;

use serde::{Serialize, Deserialize};

use self::scheduler_utils::*;

//use bento::bindings as c;
//use bento::kernel::ffi;
//use bento::kernel::raw;

//use bento::std::ffi::OsStr;
//use bento::kernel::kobj::CStr;

use spin::RwLock;

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
    pub q: Option<RwLock<VecDeque<u64>>>,
    pub map: Option<RwLock<BTreeMap<u64, u32>>>,
    pub user_q: Option<RwLock<Option<RingBuffer<UserMessage>>>>,
}

pub struct UpgradeData {
    q: Option<RwLock<VecDeque<u64>>>,
    map: Option<RwLock<BTreeMap<u64, u32>>>
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct UserMessage {
    val: i32
}

impl BentoScheduler<'_, UpgradeData, UpgradeData, UserMessage> for BentoSched {
    fn get_policy(&self) -> i32 {
        10
    }

    //fn init(&mut self) {
   // }

    fn task_new(&self, pid: u64, _runtime: u64, runnable: u16) {
        if runnable > 0 {
            let mut q = self.q.as_ref().unwrap().write();
            q.push_back(pid);
            //let mut map = self.map.as_ref().unwrap().write();
            //map.insert(pid, -1);
        }
    }

    fn task_wakeup(&self, pid: u64, _agent_data: u64, deferrable: bool,
                   _last_run_cpu: i32, wake_up_cpu: i32, _waker_cpu: i32) {
        let mut q = self.q.as_ref().unwrap().write();
        let mut map = self.map.as_ref().unwrap().write();
        if deferrable {
            q.push_back(pid);
        } else {
            q.push_front(pid);
        }
        map.insert(pid, wake_up_cpu as u32);
    }

    fn task_preempt(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64, cpu: i32,
                    _from_switchto: i8, _was_latched: i8) {
        let mut q = self.q.as_ref().unwrap().write();
        q.push_back(pid);
        let mut map = self.map.as_ref().unwrap().write();
        map.insert(pid, cpu as u32);
    }

    fn task_blocked(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64,
                    cpu: i32, _from_switchto: i8) {
        let mut q = self.q.as_ref().unwrap().write();
        // position depends on iterator state, so don't use iterator before we call position
        if let Some(idx) = q.iter().position(|&x| x == pid) {
            q.remove(idx);
        }
        let mut map = self.map.as_ref().unwrap().write();
        map.insert(pid, cpu as u32);
    }

    fn pick_next_task(&self, cpu: i32) -> Option<u64> {
        let mut q = self.q.as_ref().unwrap().write();
        if let Some(pid) = q.pop_front() {
            let map = self.map.as_ref().unwrap().read();
            if (map.get(&pid).is_none() ||
                map.get(&pid) == Some(&(cpu as u32))) {

                Some(pid)
            } else {
                //println!("can't schedule on other cpu\n");
                q.push_front(pid);
                None
            }
        } else {
            None
        }
    }

    fn select_task_rq(&self, pid: u64) -> i32 {
        let mut map = self.map.as_ref().unwrap().write();
        //*map.get(&pid).unwrap_or(&1) as i32
        match map.get(&pid) {
            None => pid as i32 % 2,
            Some(cpu) => *cpu as i32,
        }
    }

    fn migrate_task_rq(&self, pid: u64, new_cpu: i32) {
        //println!("hitting migrate_task_rq pid {} to cpu {}", pid, new_cpu);
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

    fn register_queue(&self, q: RingBuffer<UserMessage>) {
        //println!("q ptr {:?}", q.inner);
        unsafe {
        //println!("q ptr {:?}", (*q.inner).offset);
        //println!("q capacity {}", (*q.inner).capacity);
        //println!("q readptr {}", (*q.inner).readptr);
        //println!("q writeptr {}", (*q.inner).writeptr);
        let mut user_q = self.user_q.as_ref().unwrap().write();
        user_q.replace(q);

        //self.user_q = Some(RwLock::new(q));
        }
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
}

//bento::kernel_module!(
//    BentoGhostModule,
//    author: b"Bento Contributors",
//    description: b"kernel module to replace the scheduler",
//    license: b"GPL"
//);
