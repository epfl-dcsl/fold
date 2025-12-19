use alloc::{boxed::Box, fmt::Debug, vec::Vec};
use goblin::elf::program_header::PT_TLS;

use crate::{
    arena::Handle,
    dbg,
    elf::{Object, Segment},
    sysv::tls::TlsError,
    Manifold, Module, ShareMapKey,
};

pub struct TlsCollector;

#[derive(Debug, Clone)]
pub struct TlsModule {
    pub object: Handle<Object>,
    pub segment: Handle<Segment>,
}

pub const TLS_MODULES_KEY: ShareMapKey<Vec<TlsModule>> = ShareMapKey::new("tls_modules");
pub const TLS_MODULE_ID_KEY: ShareMapKey<usize> = ShareMapKey::new("tls_module_id");

impl Module for TlsCollector {
    fn name(&self) -> &'static str {
        "tls-collector"
    }

    fn process_object(
        &mut self,
        obj: Handle<Object>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn Debug>> {
        let segments: Vec<TlsModule> = manifold[obj]
            .segments
            .iter()
            .filter(|s| manifold.segments[**s].tag == PT_TLS)
            .map(|s| TlsModule {
                object: obj,
                segment: *s,
            })
            .collect();

        manifold.shared.insert_or_update(
            TLS_MODULES_KEY,
            || segments.clone(),
            |v| v.append(&mut segments.clone()),
        );

        let end = dbg!(manifold.shared.get(TLS_MODULES_KEY))
            .ok_or(TlsError::MissingSharedMapEntry(TLS_MODULES_KEY.key))?
            .len();

        manifold[obj].shared.insert(TLS_MODULE_ID_KEY, end);

        Ok(())
    }
}
