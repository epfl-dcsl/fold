use alloc::boxed::Box;
use core::fmt::Debug;

use crate::{
    sysv::tls::{
        build,
        collection::{TLS_MODULES_KEY, TLS_MODULE_ID_KEY},
        load_from_manifold, set_fs, ThreadControlBlock, TlsError,
    },
    Manifold, Module, ShareMapKey,
};

pub struct TlsAllocator;

pub const TLS_TCB_PTR: ShareMapKey<usize> = ShareMapKey::new("tls-tcb-ptr");
pub const TLS_PTR: ShareMapKey<usize> = ShareMapKey::new("tls-ptr");

impl Module for TlsAllocator {
    fn name(&self) -> &'static str {
        "tls-allocator"
    }

    fn process_manifold(&mut self, manifold: &mut Manifold) -> Result<(), Box<dyn Debug>> {
        let modules = manifold
            .shared
            .get(TLS_MODULES_KEY)
            .ok_or(TlsError::MissingSharedMapEntry(TLS_MODULES_KEY.key))?;

        let tcb = build(
            modules.len(),
            // TODO: may need to take alignment into account.
            modules.iter().map(|m| manifold[m.segment].mem_size).sum(),
        )?;

        let ptr = tcb as *mut ThreadControlBlock as usize;

        unsafe {
            set_fs(ptr);
        }

        manifold.shared.insert(TLS_TCB_PTR, ptr);

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
