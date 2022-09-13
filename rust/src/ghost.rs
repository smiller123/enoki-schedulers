use bento::kernel::string;
use bento::kernel::raw;
use bento::kernel::ffi;

use bento::println;
use bento::libc;
use bento::bindings as c;
use bento::kernel::kobj::CStr;
use bento::std::ffi::{Slice, OsStr};

use core::option::Option;

pub struct Ghost {
    /* make this a file pointer later */
    pub global_enc_file: Option<usize>,
}

impl Ghost {
    pub fn check_version() -> bool {
        Self::get_version();
        // Doesn't make sense to check version when we're in kernel
        //let mut kernel_abi_version;
        //let ret = Self::GetVersion(&mut kernel_abi_version);
        //if ret != 0 {
        //    println!("GetVersionProblem");
        //    return false;
        //}
        true
    }

    pub fn get_version() -> isize {
        let k_ghostfs_mount = "/sys/fs/ghost\0";
        if !Self::ghost_is_mounted_at(k_ghostfs_mount) {
        //    Self::mount_ghost_fs(k_ghostfs_mount);
        }
        0
    }

    pub fn ghost_is_mounted_at(mntpt: &str) -> bool {
        //let name = \0";
        let mut path = c::path::default();
        let ret = unsafe {
            ffi::rs_kern_path(mntpt.as_ptr() as *const i8, libc::O_RDWR as u32, &mut path as *mut c::path)
        };
        if ret < 0 {
            return false;
        }
        println!("hello2\n");
        let mount = unsafe {
            ffi::rs_clone_private_mount(&path as *const c::path)
        };
        if mount.is_null() {
            return false;
        }
        println!("hello3\n");
        let fstype = unsafe {
            ffi::rs_vfsmount_get_name(mount)
        };
        println!("hello4\n");
        println!("fstype {:?}", fstype);
        unsafe {
        let fstypecstr = CStr::from_raw(fstype);
        let fstypebytes = fstypecstr.to_bytes_with_nul();
        let slice = Slice::from_u8_slice(fstypebytes);
        //let fstypestr = fstypebytes.as_str();
        let fstypestr = slice.to_str().unwrap();
        println!("fstype {}", fstypestr);
        }
        //let ghost_type = "ghost\0";
        //if string::strcmp_rs(fstype, ghost_type.as_ptr() as *const raw::c_char) == 0 {
        //    return true;
        //}
        //false
        true
    }

    pub fn mount_ghost_fs(mntpt: &str) {
    }

    pub fn set_global_enclave_ctl_file(&mut self, file: *mut c::file) {
        self.global_enc_file = Some(file as usize);
    }

    pub fn create_queue(&self, elems: u32, node: u32, flags: u32, mapsize: &mut u64) -> i64 {
        //let mut offsets = c::bento_ring_offsets {
        //    head: 0,
        //    tail: 0,
        //    ring_mask: 0,
        //    flags: 0,
        //    dropped: 0,
        //    array: 0
        //};
        let mut data = c::bento_ioc_create_queue {
            elems: elems,
            flags: flags,
            mapsize: 0,
            //offsets: offsets
        };
        //let fd = 0;
        println!("file {:?}", self.global_enc_file);
        unsafe {
        println!("ioctl {}", ffi::rs_GHOST_IOC_CREATE_QUEUE() as u32);
        }
        //println!("data ptr {:?}", &mut data as *mut c::ghost_ioc_create_queue);
        let fd = unsafe {
            c::vfs_ioctl(self.global_enc_file.unwrap() as *mut c::file, ffi::rs_GHOST_IOC_CREATE_QUEUE() as u32,
                         &mut data as *mut c::bento_ioc_create_queue as u64)
        };
        *mapsize = data.mapsize;
        return fd;
    }
}
