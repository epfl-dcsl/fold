use alloc::boxed::Box;
use core::arch::asm;
use core::ffi::c_void;
use core::fmt::Debug;
use core::ptr::null_mut;
use core::slice::from_raw_parts_mut;

use rustix::mm::{mmap_anonymous, MapFlags, ProtFlags};
use zerocopy::FromBytes;

use crate::musl::{Libc, RobustList, ThreadControlBlock, MUSL_LIBC_KEY};
use crate::sysv::tls::collection::{TlsModule, TLS_MODULES_KEY};
use crate::sysv::tls::{set_fs, MuslTlsModule, TlsError, MUSL_TLS_MODULES_LL_KEY};
use crate::{Manifold, Module, ShareMapKey};

pub struct TlsAllocator;

pub const TLS_TCB: ShareMapKey<&'static mut ThreadControlBlock> = ShareMapKey::new("tls-tcb-ptr");

impl Module for TlsAllocator {
    fn name(&self) -> &'static str {
        "tls-allocator"
    }

    fn process_manifold(&mut self, manifold: &mut Manifold) -> Result<(), Box<dyn Debug>> {
        let modules = manifold.shared.get(TLS_MODULES_KEY);
        let modules = modules.iter().flat_map(|v| v.iter());

        let mut tls = alloc_tls(modules.clone(), manifold)?;
        let tls_head = setup_modules(modules, manifold, &mut tls);

        let tls_head = if let Some(tls_head) = tls_head {
            manifold.shared.insert(MUSL_TLS_MODULES_LL_KEY, tls_head);
            manifold.shared.get(MUSL_TLS_MODULES_LL_KEY).unwrap() as *const MuslTlsModule as usize
        } else {
            0
        };

        let Some(libc) = manifold.shared.get(MUSL_LIBC_KEY).cloned() else {
            log::warn!("MUSL not found, skipping TLS allocation");
            return Ok(());
        };
        let libc = libc.get_mut(&mut manifold.segments)?;
        libc.can_do_threads = 1;
        libc.tls_cnt = 1;
        libc.tls_size = tls.size;
        libc.tls_align = tls.align;
        libc.tls_head = tls_head;

        build_tcb(&mut tls, libc);

        let ptr = tls.tcb as *mut ThreadControlBlock as usize;

        unsafe {
            set_fs(ptr);
        }

        manifold.shared.insert(TLS_TCB, tls.tcb);

        Ok(())
    }
}

struct TlsBlock {
    dtv: &'static mut [usize],
    modules: &'static mut [u8],
    tcb: &'static mut ThreadControlBlock,
    size: usize,
    align: usize,
}

fn alloc_tls<'a, I>(modules: I, manifold: &Manifold) -> Result<TlsBlock, TlsError>
where
    I: Iterator<Item = &'a TlsModule> + Clone,
{
    let max_align = modules
        .clone()
        .map(|m| manifold[m.segment].align)
        .chain([align_of::<ThreadControlBlock>()])
        .max()
        .unwrap();
    let modules_size = modules.clone().map(|m| m.tls_offset).max().unwrap_or(0);
    let modules_count = modules.count();

    let dtv_size = (modules_count + 1) * size_of::<usize>();

    // Padding between the dtv and the modules part, such that the TCB is aligned with `max_align`.
    let pad = (dtv_size + modules_size).next_multiple_of(max_align) - (dtv_size + modules_size);

    // Computes the total size of the TLS block, ensuring that the alignment of modules and TCB is
    // correct.
    let tls_size = dtv_size + modules_size + pad + size_of::<ThreadControlBlock>();

    let region = unsafe {
        let addr = mmap_anonymous(
            null_mut(),
            tls_size,
            ProtFlags::READ | ProtFlags::WRITE,
            MapFlags::PRIVATE,
        )
        .map_err(TlsError::Linux)?;
        assert_ne!(addr, null_mut());

        log::info!(
            "TLS block allocated at {:#x?} with size {:#x}",
            addr,
            tls_size
        );

        from_raw_parts_mut(addr as *mut u8, tls_size)
    };

    let (dtv, region) = FromBytes::mut_from_prefix_with_elems(region, modules_count + 1).unwrap();
    let (_, region) = region.split_at_mut(pad);
    let (modules, tcb) = region.split_at_mut(modules_size);

    assert_eq!(tcb.len(), size_of::<ThreadControlBlock>());
    assert!(
        (tcb.as_ptr() as usize).trailing_zeros()
            >= align_of::<ThreadControlBlock>().trailing_zeros()
    );
    let tcb = unsafe { &mut *(tcb.as_ptr() as *mut ThreadControlBlock) };

    Ok(TlsBlock {
        dtv,
        modules,
        tcb,
        size: tls_size,
        align: max_align,
    })
}

fn setup_modules<'a, I>(
    modules: I,
    manifold: &Manifold,
    tls: &mut TlsBlock,
) -> Option<MuslTlsModule>
where
    I: Iterator<Item = &'a TlsModule> + DoubleEndedIterator,
{
    let mut tls_head = None;

    for module in modules.rev() {
        tls.dtv[0] += 1;

        let start = tls.modules.len() - module.tls_offset;
        let segment = &manifold[module.segment];

        let (data, bss) =
            &mut tls.modules[start..start + segment.mem_size].split_at_mut(segment.file_size);

        data.copy_from_slice(segment.mapping.bytes);
        bss.fill(0);

        tls.dtv[module.id] = data.as_ptr() as usize;

        tls_head = Some(Box::new(MuslTlsModule {
            next: tls_head,
            image: segment.mapping.bytes.as_ptr() as *const c_void,
            len: segment.file_size,
            size: segment.mem_size,
            align: segment.align,
            offset: module.tls_offset,
        }))
    }

    tls_head.map(|h| *h)
}

fn build_tcb(tls: &mut TlsBlock, libc: &Libc) {
    let tid: u32;
    unsafe {
        asm!(
            "syscall",
            inout("rax") 186u32 => tid,
            clobber_abi("C")
        )
    }

    *tls.tcb = ThreadControlBlock {
        tcb: tls.tcb as *mut ThreadControlBlock,
        dtv: tls.dtv.as_mut_ptr(),
        prev: &raw mut *tls.tcb,
        next: &raw mut *tls.tcb,
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
            head: &raw mut tls.tcb.robust_list.head as *mut c_void,
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
}
