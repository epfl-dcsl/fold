use alloc::{boxed::Box, fmt::Debug};
use core::{
    arch::asm,
    ffi::c_void,
    ptr::{null_mut, slice_from_raw_parts_mut},
};
use log::trace;

use rustix::{
    io::Errno,
    mm::{mmap_anonymous, MapFlags, ProtFlags},
};

use crate::{
    arena::Handle,
    elf::Object,
    musl::{Libc, RobustList, ThreadControlBlock, MUSL_LIBC_KEY},
    sysv::tls::{
        allocation::TLS_TCB,
        collection::{TLS_MODULES_KEY, TLS_MODULE_ID_KEY},
    },
    Manifold, ShareMapKey,
};

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

/// Size of the TCB in musl's implementation.
const TCB_SIZE: usize = 704;
const PAGE_SIZE: usize = 1 << 12;

fn build_dtv(module_count: usize) -> *mut usize {
    let dtv = unsafe {
        let addr = mmap_anonymous(
            null_mut(),
            (module_count + 1) * size_of::<usize>(),
            ProtFlags::READ | ProtFlags::WRITE,
            MapFlags::PRIVATE,
        )
        .unwrap();

        &mut *slice_from_raw_parts_mut(addr as *mut usize, module_count + 1)
    };

    dtv[0] = module_count;
    dtv[1..].fill(0);

    dtv.as_mut_ptr()
}

fn build_tls(
    module_count: usize,
    total_module_size: usize,
    libc: &mut Libc,
) -> Result<&'static mut ThreadControlBlock, TlsError> {
    log::info!(
        "Building TLS with tcb_size={} for {module_count} modules. Reserving {total_module_size:#x} bytes for static modules.",
        TCB_SIZE
    );

    // Ensures that the TCB is aligned on its size. Required for safely accessing its fields.
    let tcb_align = align_of::<ThreadControlBlock>();
    let tcb_aligned_size = TCB_SIZE.next_multiple_of(tcb_align);
    libc.tls_align = tcb_align;
    libc.tls_size = TCB_SIZE;

    // Compute the page-aligned size that may be required by the whole static TLS block.
    let static_size = (total_module_size + tcb_aligned_size).next_multiple_of(PAGE_SIZE);

    // Map a random region to store the TCB (and later the TLS image).
    let (tcb, tcb_addr) = unsafe {
        // TODO: NORESERVE may be wrong. The goal is to reserve virtual memory for the static
        // modules, without actually allocating physical memory.
        let addr = mmap_anonymous(
            null_mut(),
            static_size,
            ProtFlags::READ | ProtFlags::WRITE,
            MapFlags::PRIVATE | MapFlags::NORESERVE,
        )?;

        // Creates an actual mapping for the last page.
        let addr = mmap_anonymous(
            addr.add(static_size - PAGE_SIZE),
            PAGE_SIZE,
            ProtFlags::READ | ProtFlags::WRITE,
            MapFlags::PRIVATE | MapFlags::FIXED,
        )?;

        let ptr = addr.add(static_size).sub(tcb_aligned_size) as *mut ThreadControlBlock;
        log::trace!("TCB allocated at {ptr:#x?} (mapped at {addr:#x?})");

        (&mut *ptr, ptr)
    };

    let dtv = build_dtv(module_count);
    log::trace!("DTV allocated at {:#x?} for {module_count} entries", dtv);

    let tid: u32;
    unsafe {
        asm!(
            "syscall",
            inout("rax") 186u32 => tid,
            clobber_abi("C")
        )
    }

    // Build the TCB structure
    // TODO: maybe the dtv stored in TCB should be dtv[1..], since dtv[0] is the size of the vector
    *tcb = ThreadControlBlock {
        tcb: tcb_addr,
        dtv,
        prev: &raw mut *tcb,
        next: &raw mut *tcb,
        sysinfo: 0,
        stack_guard: 0xDEADBEEF_u64, // TODO: randomly generated this
        tid,
        errno: 0,
        detach_state: 0x2, // DT_JOINABLE
        cancel: 0,
        cancel_disable: 0,
        cancel_async: 0,
        flags: 0,
        map_base: null_mut(),
        map_size: 0,
        stack: null_mut(),
        stack_size: 0,
        guard_size: 0,
        result: null_mut(),
        cancel_buf: null_mut(),
        tsd: null_mut(),
        robust_list: RobustList {
            head: &raw mut tcb.robust_list.head as *mut c_void,
            off: 0,
            pending: null_mut(),
        },
        h_errno: 0,
        timer_id: 0,
        locale: &raw const libc.global_locale,
        kill_lock: 0,
        dlerror_buf: null_mut(),
        stdio_locks: null_mut(),
    };

    Ok(tcb)
}

fn load_from_manifold(
    manifold: &mut Manifold,
    obj: Handle<Object>,
    prev_offset: usize,
) -> Result<usize, Box<dyn Debug>> {
    let id = *manifold.objects[obj]
        .shared
        .get(TLS_MODULE_ID_KEY)
        .ok_or(TlsError::MissingSharedMapEntry(TLS_MODULE_ID_KEY.key))?;

    let modules = manifold
        .shared
        .get(TLS_MODULES_KEY)
        .ok_or(TlsError::MissingSharedMapEntry(TLS_MODULES_KEY.key))?;

    let module = modules.get(id - 1).ok_or(TlsError::InvalidModuleId(id))?;

    let segment = &manifold.segments[module.segment];
    let s_msize = segment.mem_size;
    let s_image = segment.mapping.bytes.as_ptr();
    let s_fsize = segment.file_size;
    let s_align = segment.align;

    let tcb = manifold
        .shared
        .get_mut(TLS_TCB)
        .ok_or(TlsError::MissingSharedMapEntry(TLS_TCB.key))?;
    let addr = load_static_module(id, segment.mapping.bytes(), s_msize, tcb, prev_offset)?;

    let libc = manifold
        .shared
        .get(MUSL_LIBC_KEY)
        .ok_or(TlsError::MissingSharedMapEntry(MUSL_LIBC_KEY.key))?
        .clone();
    let libc_mut = libc.get_mut(manifold)?;
    libc_mut.tls_size += s_msize;
    let offset = libc_mut.tls_size - TCB_SIZE;

    let tail = MuslTlsModule {
        next: None,
        image: s_image as *const c_void,
        len: s_fsize,
        size: s_msize,
        align: s_align,
        offset,
    };

    if let Some(mut node) = manifold.shared.get_mut(MUSL_TLS_MODULES_LL_KEY) {
        while node.next.is_some() {
            node = node.next.as_mut().unwrap();
        }

        node.next = Some(Box::new(tail))
    } else {
        manifold.shared.insert(MUSL_TLS_MODULES_LL_KEY, tail);
    }

    libc.get_mut(manifold)?.tls_head =
        manifold.shared.get(MUSL_TLS_MODULES_LL_KEY).unwrap() as *const MuslTlsModule as usize;

    Ok(addr)
}

fn load_static_module(
    id: usize,
    data: &[u8],
    size: usize,
    tcb: &mut ThreadControlBlock,
    prev_offset: usize,
) -> Result<usize, TlsError> {
    log::info!("Statically loading TLS module with id {id}.");

    let dtv = tcb.get_dtv();

    if dtv[id] != 0 {
        return Ok(prev_offset);
    }

    // Compute the address where the module will be loaded. It is just before the last loaded
    // module, or before the TCB if no module was loaded already.
    let mod_addr_usize = prev_offset - size;

    // The page containing prev_offset is already loaded. If the module would extend over previous
    // pages, these need to be mmapped.
    if mod_addr_usize >> 12 != prev_offset >> 12 {
        // The memory region was previously reserved while allocating the TCB. It still needs to be
        // page-aligned before calling mmap.
        let requested_addr = (mod_addr_usize & !((1 << 12) - 1)) as *mut c_void;
        let requested_size = ((prev_offset >> 12) - (mod_addr_usize >> 12)) << 12;

        let addr = unsafe {
            mmap_anonymous(
                requested_addr,
                requested_size,
                ProtFlags::READ | ProtFlags::WRITE,
                MapFlags::PRIVATE | MapFlags::FIXED,
            )
        }?;

        assert_eq!(requested_addr, addr);
    }

    let mod_slice = unsafe { &mut *slice_from_raw_parts_mut(mod_addr_usize as *mut u8, size) };

    let (tdata, tbss) = mod_slice.split_at_mut(data.len());
    tdata.copy_from_slice(data);
    tbss.fill(0);

    Ok(mod_addr_usize)
}

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
