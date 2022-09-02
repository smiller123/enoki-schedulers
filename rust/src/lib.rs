#![feature(lang_items)]
#![feature(concat_idents)]
#![feature(allocator_api)]
#![feature(alloc_error_handler)]
#![feature(alloc_layout_extra)]
#![feature(panic_info_message)]
#![feature(rustc_private)]
#![allow(improper_ctypes)]
#![feature(const_btree_new)]
#![no_std]

extern crate alloc;
extern crate bento;
extern crate spin;

pub mod enclave;
pub mod ghost;

use bento::println;

use bento::bindings as c;
use bento::kernel::ffi;
use bento::kernel::raw;
use bento::scheduler_utils::*;

use spin::RwLock;

use enclave::LocalEnclave;
use ghost::Ghost;

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;

use core::mem;

pub struct BentoGhostModule {
}

#[repr(C)]
pub struct BentoSched {
    q: Option<RwLock<VecDeque<u64>>>,
    map: Option<RwLock<BTreeMap<u64, u32>>>,
}

pub struct UpgradeData {
    q: Option<RwLock<VecDeque<u64>>>,
    map: Option<RwLock<BTreeMap<u64, u32>>>
}

pub static mut BENTO_SCHED: BentoSched = BentoSched {
    q: None,
    map: None,
};

impl bento::KernelModule for BentoGhostModule {
    fn init() -> Result<Self, i32> {
        println!("Hello from Rust");
        unsafe {
        BENTO_SCHED.q = Some(RwLock::new(VecDeque::new()));
        BENTO_SCHED.map = Some(RwLock::new(BTreeMap::new()));
        BENTO_SCHED.register();
        let this_mod = BentoGhostModule {};
        Ok(this_mod)
        }
    }
}

impl BentoScheduler<UpgradeData, UpgradeData> for BentoSched {
    fn get_policy(&self) -> i32 {
        10
    }

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
        //return None;
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
}

impl Drop for BentoGhostModule {
    fn drop(&mut self) {
        unsafe {
        println!("Saying goodbye from Rust");
        BENTO_SCHED.unregister();
        println!("Goodbye from Rust");
        }
    }
}

bento::kernel_module!(
    BentoGhostModule,
    author: b"Bento Contributors",
    description: b"kernel module to replace the scheduler",
    license: b"GPL"
);
