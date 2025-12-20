use core::{
    ffi::{c_void, CStr},
    fmt::Debug,
    ptr::{read_unaligned, slice_from_raw_parts_mut},
};

use alloc::boxed::Box;

use crate::{arena::Handle, elf::Object, Manifold, Module, ShareMapKey};

pub type Sysinfo = usize;

#[repr(C)]
pub struct RobustList {
    pub head: *mut c_void,
    pub off: u64,
    pub pending: *mut c_void,
}

#[repr(C)]
pub struct ThreadControlBlock {
    pub tcb: *mut ThreadControlBlock,
    pub dtv: *mut usize,
    pub prev: *mut ThreadControlBlock,
    pub next: *mut ThreadControlBlock,
    pub sysinfo: Sysinfo,
    pub stack_guard: u64,

    // Musl specific entries
    pub tid: u32,
    pub errno: u32,
    pub detach_state: u32,
    pub cancel: u32,
    pub cancel_disable: u8,
    pub cancel_async: u8,
    pub flags: u8,
    pub map_base: *mut u8,
    pub map_size: usize,
    pub stack: *mut c_void,
    pub stack_size: usize,
    pub guard_size: usize,
    pub result: *mut c_void,
    pub cancel_buf: *mut c_void,
    pub tsd: *mut *mut c_void,
    pub robust_list: RobustList,
    pub h_errno: u32,
    pub timer_id: u32,
    pub locale: *mut c_void,
    pub kill_lock: u32,
    pub dlerror_buf: *mut u8,
    pub stdio_locks: *mut u8,
}

impl ThreadControlBlock {
    pub fn get_dtv(&mut self) -> &mut [usize] {
        unsafe {
            let size = read_unaligned(self.dtv);
            &mut *slice_from_raw_parts_mut(self.dtv, size + 1)
        }
    }
}

pub struct Libc {
    pub can_do_threads: u8,
    pub threaded: u8,
    pub secure: u8,
    pub need_locks: i8,
    pub trheads_minus_1: u32,
    pub auxv: *mut usize,
    pub tls_head: *mut c_void,
    pub tls_size: usize,
    pub tls_align: usize,
    pub tls_cnt: usize,
    pub page_size: usize,
    pub global_local: Locale,
}

pub struct Locale {
    pub cat: [*const LocaleMap; 6],
}

pub struct LocaleMap {
    pub map: *const c_void,
    pub map_size: usize,
    pub name: [u8; 24],
    pub next: *const LocaleMap,
}

pub struct MuslLocator;

pub const MUSL_LIBC_KEY: ShareMapKey<*mut Libc> = ShareMapKey::new("musl-libc");
pub const MUSL_SYSINFO_KEY: ShareMapKey<*mut Sysinfo> = ShareMapKey::new("musl-sysinfo");

fn locate_and_insert_sym<T>(
    manifold: &mut Manifold,
    obj: Handle<Object>,
    name: &CStr,
    key: ShareMapKey<*mut T>,
) where
    T: 'static,
{
    if let Ok((_, sym)) = manifold.find_symbol(name, obj) {
        let addr = manifold[obj].mapping.bytes.as_ptr().addr() + sym.st_value as usize;

        log::trace!("Found {} at {addr:#x}", name.to_string_lossy());

        manifold.shared.insert(key, addr as *mut T);
    } else {
        log::warn!("Symbol {} not found", name.to_string_lossy())
    }
}

impl Module for MuslLocator {
    fn name(&self) -> &'static str {
        "musl-locator"
    }

    fn process_manifold(&mut self, manifold: &mut Manifold) -> Result<(), Box<dyn Debug>> {
        let Some((obj, _)) = manifold
            .objects
            .enumerate()
            .find(|(_, o)| o.display_path().ends_with("libc.so"))
        else {
            log::warn!("Unable to find libc.so object");
            return Ok(());
        };

        locate_and_insert_sym(manifold, obj, c"__libc", MUSL_LIBC_KEY);
        locate_and_insert_sym(manifold, obj, c"__sysinfo", MUSL_SYSINFO_KEY);

        Ok(())
    }
}
