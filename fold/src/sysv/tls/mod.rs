use alloc::boxed::Box;
use alloc::fmt::Debug;
use core::arch::asm;
use core::ffi::c_void;

use log::trace;
use rustix::io::Errno;

use crate::ShareMapKey;

pub mod allocation;
pub mod collection;
pub mod relocation;

unsafe fn set_fs(addr: usize) {
    trace!("Set fs register to 0x{addr:x}");
    let syscall_number: u64 = 158; // arch_prctl syscall
    let arch_set_fs: u64 = 0x1002; // set FS

    asm!(
        "syscall",
        inout("rax") syscall_number => _,
        in("rdi") arch_set_fs,
        in("rsi") addr,
        lateout("rcx") _, lateout("r11") _,
    );
}

struct TLSModule<'a> {
    module_id: usize,
    must_be_static: bool,
    data: &'a [u8],
    size: usize,
}

#[derive(Debug)]
#[repr(C)]
pub struct MuslTlsModule {
    next: Option<Box<MuslTlsModule>>,
    image: *const c_void,
    len: usize,
    size: usize,
    align: usize,
    offset: usize,
}

pub const MUSL_TLS_MODULES_LL_KEY: ShareMapKey<MuslTlsModule> =
    ShareMapKey::new("musl-tls-modules");

const PAGE_SIZE: usize = 1 << 12;

#[derive(Debug)]
pub enum TlsError {
    Linux(Errno),
    InvalidModuleId(usize),
    MissingSharedMapEntry(&'static str),
}

impl From<Errno> for TlsError {
    fn from(value: Errno) -> Self {
        Self::Linux(value)
    }
}

impl From<TlsError> for Box<dyn Debug> {
    fn from(value: TlsError) -> Self {
        Box::new(value) as Self
    }
}
