use alloc::boxed::Box;
use alloc::fmt::Debug;
use alloc::vec;
use alloc::vec::Vec;

use goblin::elf::program_header::PT_TLS;

use crate::arena::Handle;
use crate::elf::{Object, Segment};
use crate::sysv::tls::TlsError;
use crate::{Manifold, Module, ShareMapKey, INITIAL_ELF_KEY};

pub struct TlsCollector {
    last_offset: usize,
    module_id_alloc: usize,
}

impl TlsCollector {
    pub fn new() -> Self {
        Self {
            last_offset: 0,
            module_id_alloc: 2,
        }
    }
}

impl Default for TlsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct TlsModule {
    pub id: usize,
    pub tls_offset: usize,
    pub object: Handle<Object>,
    pub segment: Handle<Segment>,
}

pub const TLS_MODULES_KEY: ShareMapKey<Vec<TlsModule>> = ShareMapKey::new("tls-modules");
pub const TLS_MODULE_KEY: ShareMapKey<TlsModule> = ShareMapKey::new("tls-module");

impl Module for TlsCollector {
    fn name(&self) -> &'static str {
        "tls-collector"
    }

    fn process_object(
        &mut self,
        obj: Handle<Object>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn Debug>> {
        // Locates the TLS module of this object; returning immediately if there is none.
        let Some(hseg) = manifold[obj]
            .segments
            .iter()
            .find(|s| manifold.segments[**s].tag == PT_TLS)
            .copied()
        else {
            return Ok(());
        };
        let segment = &manifold.segments[hseg];

        let tls_offset = (self.last_offset + segment.mem_size).next_multiple_of(segment.align);

        let initial_elf = manifold
            .shared
            .get(INITIAL_ELF_KEY)
            .ok_or(TlsError::MissingSharedMapEntry(INITIAL_ELF_KEY.key))?;

        let id = if *initial_elf == obj {
            1
        } else {
            let id = self.module_id_alloc;
            self.module_id_alloc += 1;
            id
        };

        let module = TlsModule {
            id,
            tls_offset,
            object: obj,
            segment: hseg,
        };

        manifold.shared.insert_or_update(
            TLS_MODULES_KEY,
            || vec![module.clone()],
            |v| v.push(module.clone()),
        );
        manifold[obj].shared.insert(TLS_MODULE_KEY, module);

        self.last_offset = tls_offset;

        Ok(())
    }
}
