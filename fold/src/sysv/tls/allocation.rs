use alloc::boxed::Box;
use core::fmt::Debug;

use crate::{
    musl::MUSL_LIBC_KEY,
    sysv::tls::{
        build_tls,
        collection::{TLS_MODULES_KEY, TLS_MODULE_ID_KEY},
        load_from_manifold, set_fs, ThreadControlBlock,
    },
    Manifold, Module, ShareMapKey,
};

pub struct TlsAllocator;

pub const TLS_TCB: ShareMapKey<&'static mut ThreadControlBlock> = ShareMapKey::new("tls-tcb-ptr");
pub const TLS_PTR: ShareMapKey<usize> = ShareMapKey::new("tls-ptr");

impl Module for TlsAllocator {
    fn name(&self) -> &'static str {
        "tls-allocator"
    }

    fn process_manifold(&mut self, manifold: &mut Manifold) -> Result<(), Box<dyn Debug>> {
        let modules = manifold.shared.get(TLS_MODULES_KEY);
        let module_count = modules.map(|m| m.len()).unwrap_or_default();

        // TODO: needs to take alignment into account.
        let total_module_size = modules
            .iter()
            .flat_map(|v| v.iter())
            .map(|m| manifold[m.segment].mem_size)
            .sum();

        let libc = manifold.shared.get(MUSL_LIBC_KEY).unwrap().clone();
        let libc_mut = libc.get_mut(manifold)?;
        libc_mut.can_do_threads = 1;
        libc_mut.tls_cnt = 1;

        let tcb = build_tls(module_count, total_module_size, libc_mut)?;

        let ptr = tcb as *mut ThreadControlBlock as usize;

        unsafe {
            set_fs(ptr);
        }

        manifold.shared.insert(TLS_TCB, tcb);

        let first_obj = manifold.objects.handle_generator().next().unwrap();
        let ptr = if manifold[first_obj].shared.get(TLS_MODULE_ID_KEY).is_some() {
            load_from_manifold(manifold, first_obj, ptr)?
        } else {
            ptr
        };

        manifold.shared.insert(TLS_PTR, ptr);

        Ok(())
    }
}
