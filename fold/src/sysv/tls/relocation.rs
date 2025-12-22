use alloc::{boxed::Box, vec::Vec};
use core::{cell::LazyCell, fmt::Debug, ptr::write_unaligned};
use goblin::{
    elf::reloc::{
        R_X86_64_DTPMOD64, R_X86_64_DTPOFF32, R_X86_64_DTPOFF64, R_X86_64_GOTTPOFF, R_X86_64_TLSGD,
        R_X86_64_TLSLD, R_X86_64_TPOFF32, R_X86_64_TPOFF64,
    },
    elf64::reloc::{self, Rela},
};

use crate::{
    arena::Handle,
    elf::{ElfItemIterator, Section, SectionT},
    musl::ThreadControlBlock,
    sysv::{
        loader::SYSV_LOADER_BASE_ADDR,
        tls::{
            allocation::{TLS_PTR, TLS_TCB},
            load_from_manifold, TlsError,
        },
    },
    Manifold, Module,
};

pub struct TlsRelocator;

const TLS_RELOCS: &[u32] = &[
    R_X86_64_DTPMOD64,
    R_X86_64_DTPOFF32,
    R_X86_64_DTPOFF64,
    R_X86_64_TPOFF32,
    R_X86_64_TPOFF64,
    R_X86_64_GOTTPOFF,
    R_X86_64_TLSGD,
    R_X86_64_TLSLD,
];

impl Module for TlsRelocator {
    fn name(&self) -> &'static str {
        "tls-relocation"
    }

    fn process_section(
        &mut self,
        section: Handle<Section>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn Debug>> {
        let tp = *manifold
            .shared
            .get_mut(TLS_TCB)
            .ok_or(TlsError::MissingSharedMapEntry(TLS_TCB.key))?
            as *mut ThreadControlBlock as usize;
        let section = &manifold[section];
        let mut tls_ptr = *manifold
            .shared
            .get(TLS_PTR)
            .ok_or(TlsError::MissingSharedMapEntry(TLS_PTR.key))?;

        let base = manifold[section.obj]
            .shared
            .get(SYSV_LOADER_BASE_ADDR)
            .copied()
            .ok_or(TlsError::MissingSharedMapEntry(SYSV_LOADER_BASE_ADDR.key))?
            as *mut u8;

        let static_relocs = ElfItemIterator::<Rela>::from_section(section)
            .filter_map(|rela| {
                let r#type = reloc::r_type(rela.r_info);
                let sym = reloc::r_sym(rela.r_info);

                if !TLS_RELOCS.contains(&r#type) {
                    return None;
                }

                // Get the symbol's name
                let name = LazyCell::new(|| {
                    section
                        .get_linked_section(manifold)
                        .ok()
                        .and_then(|s| s.as_dynamic_symbol_table().ok())
                        .and_then(|s| s.get_symbol_name(sym as usize, manifold).ok())
                        .unwrap()
                });

                let orig = LazyCell::new(|| manifold.find_symbol(*name, section.obj).unwrap());

                log::info!(
                    "Processing reloc {:#x} with symbol \"{:#}\"",
                    r#type,
                    name.to_string_lossy()
                );

                match r#type {
                    R_X86_64_TPOFF32 => {
                        let (section, sym) = *orig;
                        Some((rela.r_offset, section.obj, sym, 32))
                    }
                    R_X86_64_TPOFF64 => {
                        let (section, sym) = *orig;
                        Some((rela.r_offset, section.obj, sym, 64))
                    }
                    R_X86_64_DTPMOD64 | R_X86_64_DTPOFF32 | R_X86_64_DTPOFF64
                    | R_X86_64_GOTTPOFF | R_X86_64_TLSGD | R_X86_64_TLSLD => None,
                    _ => unreachable!(),
                }
            })
            .collect::<Vec<_>>();

        for (offset, obj, sym, size) in static_relocs {
            let addr = unsafe { base.add(offset as usize) };

            tls_ptr = load_from_manifold(manifold, obj, tls_ptr).unwrap();

            log::trace!("Found symbol at offset {}", sym.st_value);
            let offset = (tp - tls_ptr + sym.st_value as usize) as u64;

            match size {
                64 => unsafe {
                    write_unaligned(addr as *mut u64, offset.wrapping_neg());
                },
                32 => unsafe {
                    write_unaligned(addr as *mut u32, (offset as u32).wrapping_neg());
                },
                _ => unreachable!(),
            }
        }

        manifold.shared.insert(TLS_PTR, tls_ptr);

        Ok(())
    }
}
