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

struct BentoGhostModule {
    local_enclave: LocalEnclave,
    agent: c::ghost_agent_type,
}

impl bento::KernelModule for BentoGhostModule {
    //#[no_mangle]
    //pub fn rust_main() {
    fn init() -> Result<Self, i32> {
        println!("Hello from Rust");
        let _ = Ghost::check_version();
        let mut ghost = Ghost {
            global_enc_file: None,
        };
        let mut map_size = 0;
        let local_enclave = LocalEnclave::new(ghost);
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
        c::register_ghost_agent(&mut agent as *mut c::ghost_agent_type);
        Ok(BentoGhostModule{ local_enclave: local_enclave, agent: agent })
        }
    }
}

pub extern "C" fn parse_message(type_: i32, msglen: i32, barrier: u32, payload: *mut raw::c_void, payload_size: i32) {
    println!("got message {}", type_);
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
