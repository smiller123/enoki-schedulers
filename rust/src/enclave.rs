use bento::bindings as c;
use bento::kernel::ffi;
use bento::kernel::raw;
use bento::libc;
use bento::println;

use ::ghost::*;

pub struct LocalEnclave {
    /* make this a real file pointer later */
    ctl_file: usize,
    pub ghost: Ghost,
}

impl LocalEnclave {
    pub fn new(mut ghost: Ghost) -> Self {
        let enc_file = Self::create_and_attach_to_enclave(&mut ghost);
        Self {
            ctl_file: enc_file as usize,
            ghost: ghost,
        }
    }

    pub fn create_and_attach_to_enclave(ghost: &mut Ghost) -> *mut c::file {
        let enc_file = Self::make_next_enclave().unwrap();
        Self::common_init(ghost, enc_file);
        return enc_file;
    }

    pub fn make_next_enclave() -> Result<*mut c::file, i32> {
        let name = "/sys/fs/ghost/ctl\0";
        let mut path = c::path::default();
        unsafe {
            ffi::rs_kern_path(name.as_ptr() as *const i8, libc::O_WRONLY as u32, &mut path as *mut c::path);
        }
        let file = unsafe {
            c::dentry_open(&path, libc::O_WRONLY, ffi::rs_current_cred())
        };
        let mut id = 1;
        loop {
            let write_str = alloc::format!("create {}\0", id);
            let mut write_ptr: i64 = 0;
            let ret = unsafe {
                c::kernel_write(file, write_str.as_ptr() as *const raw::c_void, write_str.as_bytes().len(), &mut write_ptr as *mut i64)
            };
            if ret == write_str.as_bytes().len() as isize {
                println!("made enclave {}", write_str);
                break;
            }
            if ret == -libc::EEXIST as isize {
                id += 1;
                continue;
            }
            println!("enclave failure {}", ret);
            unsafe {
                c::filp_close(file, 0 as *mut raw::c_void)
            };
            return Err(-1);
            //ffi::make_enclave(id)
        }
        unsafe {
            c::filp_close(file, 0 as *mut raw::c_void)
        };
        let enc_name = alloc::format!("/sys/fs/ghost/enclave_{}\0", id);
        let mut enc_path = c::path::default();
        unsafe {
            ffi::rs_kern_path(enc_name.as_ptr() as *const i8, libc::O_WRONLY as u32, &mut enc_path as *mut c::path);
        }
        let enc_file = unsafe {
            c::dentry_open(&enc_path, libc::O_WRONLY, ffi::rs_current_cred())
        };
        return Ok(enc_file);
    }

    pub fn common_init(ghost: &mut Ghost, enc_file: *mut c::file) {
        ghost.set_global_enclave_ctl_file(enc_file);
    }
}
