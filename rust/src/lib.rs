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

use alloc::boxed::Box;
use alloc::collections::vec_deque::VecDeque;
use bento::println;

use bento::bindings as c;
use bento::kernel::ffi;

use enclave::LocalEnclave;
use ghost::Ghost;

struct BentoGhostModule {
    local_enclave: LocalEnclave,
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
        local_enclave.ghost.create_queue(1, 0, 0, &mut map_size);
        Ok(BentoGhostModule{ local_enclave: local_enclave })
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
