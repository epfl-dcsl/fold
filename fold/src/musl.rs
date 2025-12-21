use core::{
    ffi::{c_void, CStr},
    fmt::Debug,
    marker::PhantomData,
    ops::Range,
    ptr::{read_unaligned, slice_from_raw_parts_mut},
};

use alloc::boxed::Box;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{
    arena::Handle, elf::Object, sysv::loader::SYSV_LOADER_MAPPING, Manifold, Module, ShareMapKey,
};

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
    pub locale: *const Locale,
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

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct Libc {
    pub can_do_threads: u8,
    pub threaded: u8,
    pub secure: u8,
    pub need_locks: i8,
    pub trheads_minus_1: u32,
    pub auxv: usize,
    pub tls_head: usize,
    pub tls_size: usize,
    pub tls_align: usize,
    pub tls_cnt: usize,
    pub page_size: usize,
    pub global_locale: Locale,
}

#[derive(Debug, Clone, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct Locale {
    pub cat: [usize; 6],
}

#[derive(Debug, Clone, Copy)]
enum MuslLocatorError {
    MutObjectNotFound,
    Conversion,
}

pub struct MuslLocator;

pub const MUSL_LIBC_KEY: ShareMapKey<MuslObjectIdx<Libc>> = ShareMapKey::new("musl-libc");
pub const MUSL_SYSINFO_KEY: ShareMapKey<MuslObjectIdx<Sysinfo>> = ShareMapKey::new("musl-sysinfo");

#[derive(Debug, Clone)]
pub struct MuslObjectIdx<T> {
    range: Range<usize>,
    musl_obj: Handle<Object>,
    data: PhantomData<T>,
}

impl<T> MuslObjectIdx<T>
where
    T: FromBytes + KnownLayout + Immutable + IntoBytes + 'static,
{
    pub fn get(&self, manifold: &Manifold) -> Result<&T, Box<dyn Debug>> {
        T::ref_from_bytes(&manifold.objects[self.musl_obj].mapping.bytes[self.range.clone()])
            .map_err(|e| Box::new(e) as Box<dyn Debug>)
    }
    pub fn get_mut<'a>(&self, manifold: &'a mut Manifold) -> Result<&'a mut T, Box<dyn Debug>> {
        let obj = &mut manifold.objects[self.musl_obj];

        let segment = obj
            .segments
            .iter()
            .find(|s| {
                let s = &manifold.segments[**s];
                s.offset <= self.range.start && self.range.end <= s.offset + s.mem_size
            })
            .ok_or_else(|| Box::new(MuslLocatorError::MutObjectNotFound) as Box<dyn Debug>)?;

        let seg_off = manifold.segments[*segment].offset;

        let mapping = manifold.segments[*segment]
            .shared
            .get_mut(SYSV_LOADER_MAPPING)
            .ok_or_else(|| Box::new(MuslLocatorError::MutObjectNotFound) as Box<dyn Debug>)?;

        T::mut_from_bytes(
            &mut mapping.bytes_mut()[self.range.start - seg_off..self.range.end - seg_off],
        )
        .map_err(|_| Box::new(MuslLocatorError::Conversion) as Box<dyn Debug>)
    }
}

fn locate_and_insert_sym<T>(
    manifold: &mut Manifold,
    obj: Handle<Object>,
    name: &CStr,
    key: ShareMapKey<MuslObjectIdx<T>>,
) where
    T: 'static,
{
    if let Ok((_, sym)) = manifold.find_symbol(name, obj) {
        log::trace!("Found {} at {:#x}", name.to_string_lossy(), sym.st_value,);
        manifold.shared.insert(
            key,
            MuslObjectIdx {
                range: sym.st_value as usize..(sym.st_value + sym.st_size) as usize,
                musl_obj: obj,
                data: PhantomData,
            },
        );
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
