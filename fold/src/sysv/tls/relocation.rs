use alloc::boxed::Box;
use core::cell::LazyCell;
use core::fmt::Debug;
use core::ptr::write_unaligned;

use goblin::elf::reloc::{
    R_X86_64_DTPMOD64, R_X86_64_DTPOFF32, R_X86_64_DTPOFF64, R_X86_64_GOTTPOFF, R_X86_64_TLSGD,
    R_X86_64_TLSLD, R_X86_64_TPOFF32, R_X86_64_TPOFF64,
};
use goblin::elf64::reloc::{self, Rela};

use crate::arena::Handle;
use crate::elf::{ElfItemIterator, Section, SectionT};
use crate::sysv::loader::SYSV_LOADER_BASE_ADDR;
use crate::sysv::tls::collection::TLS_MODULE_KEY;
use crate::sysv::tls::TlsError;
use crate::{Manifold, Module};

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
        let section = &manifold[section];

        let base = manifold[section.obj]
            .shared
            .get(SYSV_LOADER_BASE_ADDR)
            .copied()
            .ok_or(TlsError::MissingSharedMapEntry(SYSV_LOADER_BASE_ADDR.key))?
            as *mut u8;

        for rela in ElfItemIterator::<Rela>::from_section(section) {
            let addr = unsafe { base.add(rela.r_offset as usize) };
            let r#type = reloc::r_type(rela.r_info);
            let sym = reloc::r_sym(rela.r_info);

            if !TLS_RELOCS.contains(&r#type) {
                continue;
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

            let tls_offset = LazyCell::new(|| {
                manifold[orig.0.obj]
                    .shared
                    .get(TLS_MODULE_KEY)
                    .expect("TLS reloc found for object with no TLS module")
                    .tls_offset
            });

            log::info!(
                "Processing reloc {:#x} with symbol \"{:#}\"",
                r#type,
                name.to_string_lossy()
            );

            match r#type {
                R_X86_64_TPOFF32 => {
                    let offset = (tls_offset.wrapping_neg() + orig.1.st_value as usize) as u32;
                    unsafe {
                        write_unaligned(addr as *mut u32, offset);
                    }
                }
                R_X86_64_TPOFF64 => {
                    let offset = (tls_offset.wrapping_neg() + orig.1.st_value as usize) as u64;
                    unsafe {
                        write_unaligned(addr as *mut u64, offset);
                    }
                }
                R_X86_64_DTPMOD64 | R_X86_64_DTPOFF32 | R_X86_64_DTPOFF64 | R_X86_64_GOTTPOFF
                | R_X86_64_TLSGD | R_X86_64_TLSLD => {}
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}
