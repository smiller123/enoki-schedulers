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

pub struct BentoGhostModule {
}

#[repr(C)]
pub struct BentoSched {
    q: Option<RwLock<VecDeque<u64>>>,
    map: Option<RwLock<BTreeMap<u64, i32>>>,
}

pub static mut BENTO_SCHED: BentoSched = BentoSched {
    q: None,
    map: None
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

impl BentoScheduler for BentoSched {
    fn get_policy(&self) -> i32 {
        10
    }

    fn task_new(&self, pid: u64, _runtime: u64, runnable: u16) {
        if runnable > 0 {
            let mut q = self.q.as_ref().unwrap().write();
            q.push_back(pid);
            let mut map = self.map.as_ref().unwrap().write();
            map.insert(pid, -1);
        }
    }

    fn task_wakeup(&self, pid: u64, _agent_data: u64, deferrable: i8,
                   _last_run_cpu: i32, wake_up_cpu: i32, _waker_cpu: i32) {
        let mut q = self.q.as_ref().unwrap().write();
        let mut map = self.map.as_ref().unwrap().write();
        if deferrable == 0 {
            q.push_front(pid);
            map.insert(pid, wake_up_cpu);
        } else {
            q.push_back(pid);
            map.insert(pid, wake_up_cpu);
        }
    }

    fn task_preempt(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64, cpu: i32,
                    _from_switchto: i8, _was_latched: i8) {
        let mut q = self.q.as_ref().unwrap().write();
        q.push_back(pid);
        let mut map = self.map.as_ref().unwrap().write();
        map.insert(pid, cpu);
    }

    fn task_blocked(&self, pid: u64, _runtime: u64, _cpu_seqnum: u64,
                    cpu: i32, _from_switchto: i8) {
        let mut q = self.q.as_ref().unwrap().write();
        // position depends on iterator state, so don't use iterator before we call position
        if let Some(idx) = q.iter().position(|&x| x == pid) {
            q.remove(idx);
        }
        let mut map = self.map.as_ref().unwrap().write();
        map.insert(pid, cpu);
    }

    fn pick_next_task(&self, cpu: i32, ret: &mut i32) {
        let mut q = self.q.as_ref().unwrap().write();
        if let Some(pid) = q.pop_front() {
            let map = self.map.as_ref().unwrap().read();
            if (map.get(&pid).is_none() ||
                map.get(&pid) == Some(&-1) ||
                map.get(&pid) == Some(&cpu)) {

                *ret = pid as i32;
            } else {
                //println!("can't schedule on other cpu\n");
                //println!("scheduling on other cpu\n");
                //*ret = pid as i32;
                q.push_front(pid);
            }
        } else {
            *ret = 0;
        }
    }

    fn select_task_rq(&self, pid: u64, retval: &mut i32) {
        //println!("hitting select_task_rq");
        let mut map = self.map.as_ref().unwrap().write();
        if let Some(-1) = map.get(&pid) {
            *retval = 0;
        } else if let Some(cpu) = map.get(&pid) {
            //println!("moving from cpu {} to cpu {}", *cpu, new_cpu);
            *retval = *cpu;
        } else {
            *retval = 0;
        }
    }

    fn migrate_task_rq(&self, pid: u64, new_cpu: i32) {
        //println!("hitting migrate_task_rq pid {} to cpu {}", pid, new_cpu);
    }

    fn balance(&self) {
        //println!("hitting balance");
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
