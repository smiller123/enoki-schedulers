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

pub mod enclave;
pub mod ghost;

use bento::println;

use bento::bindings as c;
use bento::kernel::ffi;
use bento::kernel::raw;

use enclave::LocalEnclave;
use ghost::Ghost;

use alloc::collections::vec_deque::VecDeque;
use alloc::collections::btree_map::BTreeMap;

struct BentoGhostModule {
    //local_enclave: LocalEnclave,
    //agent: alloc::boxed::Box<c::ghost_agent_type>,
}

static mut TASK_ID: u64 = 0;
static mut Q: Option<VecDeque<u64>> = None;
static mut MAP: Option<BTreeMap<u64, i32>> = None;

impl bento::KernelModule for BentoGhostModule {
    //#[no_mangle]
    //pub fn rust_main() {
    fn init() -> Result<Self, i32> {
        println!("Hello from Rust");
        //let _ = Ghost::check_version();
        //let mut ghost = Ghost {
        //    global_enc_file: None,
        //};
        //let mut map_size = 0;
        //let local_enclave = LocalEnclave::new(ghost);
        //let q_fd = local_enclave.ghost.create_queue(1, 0, 0, &mut map_size);
        //let q_fd_struct = unsafe {
        //    ffi::rs_fdget(q_fd as u32)
        //};
        //let ghost_q = unsafe {
        //    ffi::rs_fd_to_queue(q_fd_struct)
        //};
        //unsafe {
        //    (*ghost_q).process_message = Some(parse_message);
        //}
        unsafe {
        //let agent_name = "bento\0";
        let mut agent = c::ghost_agent_type {
            policy: 10,
            process_message: Some(parse_message),
            owner: 0 as *mut c::module,
            next: 0 as *mut c::ghost_agent_type
        };
        let mut agent_box = alloc::boxed::Box::new(agent);
        Q = Some(VecDeque::new());
        MAP = Some(BTreeMap::new());
        c::register_ghost_agent(&mut agent_box as &mut c::ghost_agent_type as *mut c::ghost_agent_type);
        //let agent_box = alloc::boxed::Box::from_raw(&mut agent as *mut c::ghost_agent_type);
        alloc::boxed::Box::leak(agent_box);
        //Ok(BentoGhostModule{ local_enclave: local_enclave})
        Ok(BentoGhostModule{ })
        }
    }
}

pub extern "C" fn parse_message(type_: i32, msglen: i32, barrier: u32, payload: *mut raw::c_void, payload_size: i32, retval: *mut i32) {
    if type_ != c::MSG_PNT as i32 {
        //println!("got message {}", type_);
    }
    unsafe {
        if type_ == c::MSG_TASK_NEW as i32 {
            let payload_data = payload as *const c::ghost_msg_payload_task_new;
            TASK_ID = (*payload_data).pid;
            if (*payload_data).runnable > 0 {
                let q = Q.as_mut().unwrap();
                q.push_back((*payload_data).pid);
                let map = MAP.as_mut().unwrap();
                map.insert((*payload_data).pid, -1);
            }
            //println!("new task id {}", (*payload_data).pid);
        }
        if type_ == c::MSG_TASK_WAKEUP as i32{
            let payload_data = payload as *const c::ghost_msg_payload_task_wakeup;
            TASK_ID = (*payload_data).pid;
            let q = Q.as_mut().unwrap();
            let map = MAP.as_mut().unwrap();
            if (*payload_data).deferrable == 0 {
                q.push_front((*payload_data).pid);
                map.insert((*payload_data).pid, (*payload_data).wake_up_cpu);
            } else {
                q.push_back((*payload_data).pid);
                map.insert((*payload_data).pid, (*payload_data).wake_up_cpu);
            }
            //println!("wakeup task id {}, wake cpu {}, last cpu {}", (*payload_data).pid, (*payload_data).wake_up_cpu, (*payload_data).last_ran_cpu);
        }
        if type_ == c::MSG_TASK_PREEMPT as i32 {
            let payload_data = payload as *const c::ghost_msg_payload_task_preempt;
            TASK_ID = (*payload_data).pid;
            let q = Q.as_mut().unwrap();
            q.push_back((*payload_data).pid);
            let map = MAP.as_mut().unwrap();
            map.insert((*payload_data).pid, (*payload_data).cpu);
            //println!("preempt task id {}, cpu {}", (*payload_data).pid, (*payload_data).cpu);
        }
        if type_ == c::MSG_TASK_BLOCKED as i32 {
            let payload_data = payload as *const c::ghost_msg_payload_task_blocked;
            let blk_pid = (*payload_data).pid;
            let q = Q.as_mut().unwrap();
            // position depends on iterator state, so don't use iterator before we call position
            //println!("blocked task id {}, cpu {}", (*payload_data).pid, (*payload_data).cpu);
            if let Some(idx) = q.iter().position(|&x| x == blk_pid) {
                q.remove(idx);
            }
            let map = MAP.as_mut().unwrap();
            map.insert((*payload_data).pid, (*payload_data).cpu);
        }
    //*retval = 42;
        if type_ == c::MSG_PNT as i32{
            let payload_data = payload as *const c::ghost_msg_payload_pnt;
            //*retval = TASK_ID as i32;
            //if let None = Q.as_mut() {
                //println!("things are fucked\n");
            //}
            //if (*payload_data).cpu != 0 {
            //    *retval = 0;
            //    return;
            //}
            let q = Q.as_mut().unwrap();
            if let Some(pid) = q.pop_front() {
                let map = MAP.as_mut().unwrap();
                if (map.get(&pid).is_none() ||
                    map.get(&pid) == Some(&-1) ||
                    map.get(&pid) == Some(&(*payload_data).cpu)) {

                    *retval = pid as i32;
                    //println!("scheduling {} on {}, get {:?}", pid, (*payload_data).cpu, map.get(&pid));
                } else {
                    println!("can't schedule on other cpu\n");
                    q.push_front(pid);
                }
            } else {
                *retval = 0;
            }
            TASK_ID = 0;
        }
    }
}

impl Drop for BentoGhostModule {
    fn drop(&mut self) {
        println!("Goodbye from Rust");
    //#[no_mangle]
    //pub fn rust_exit() {
    }
}

bento::kernel_module!(
    BentoGhostModule,
    author: b"Bento Contributors",
    description: b"kernel module to replace the scheduler",
    license: b"GPL"
);
