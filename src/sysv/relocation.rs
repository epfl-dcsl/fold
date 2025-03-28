use alloc::boxed::Box;
use core::slice;

use goblin::elf::reloc::{R_X86_64_64, R_X86_64_COPY, R_X86_64_JUMP_SLOT, R_X86_64_RELATIVE};
use goblin::elf64::reloc::Rela;
use log::info;

use crate::elf::ElfItemIterator;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::object::section::SectionT;
use crate::sysv::error::SysvError;
use crate::Handle;

macro_rules! apply_reloc {
    ($addr:expr, $value:expr, $type:ty) => {
        unsafe { core::ptr::write_unaligned($addr as *mut $type, $value as $type) };
    };
}

pub struct SysvReloc {}

impl SysvReloc {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SysvReloc {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SysvReloc {
    fn name(&self) -> &'static str {
        "sysv-reloc"
    }

    fn process_section(
        &mut self,
        section_handle: Handle<crate::Section>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        let section = &manifold.sections[section_handle];
        log::info!(
            "Process relocation of section {:?}...",
            section.get_display_name(manifold).unwrap_or_default()
        );

        let obj = &manifold.objects[section.obj];
        let base = obj
            .pie_load_offset
            .ok_or(SysvError::RelaSectionWithoutVirtualAdresses)? as *mut u8;

        for rela in ElfItemIterator::<Rela>::from_section(section) {
            let addr = unsafe { base.add(rela.r_offset as usize) };
            let r#type = rela.r_info as u32;
            let sym = (rela.r_info >> 32) as u32;

            let b = base as i64;
            let a = rela.r_addend as i64;
            let s = b + section
                .get_linked_section(manifold)?
                .as_dynamic_symbol_table()?
                .get_entry(sym as usize)?
                .st_value as i64;

            match r#type {
                R_X86_64_64 => {
                    apply_reloc!(addr, (s + a), u64);
                }
                R_X86_64_COPY => {
                    let name = section
                        .get_linked_section(manifold)?
                        .as_dynamic_symbol_table()?
                        .get_symbol(sym as usize, manifold)?;

                    'find_symbol: for (_, lib_obj) in
                        manifold.objects.enumerate().filter(|s| s.0 != section.obj)
                    {
                        for lib_section in &lib_obj.sections {
                            // Get the section as DYNSYM, or skip if it has another type.
                            let Ok(lib_section) = manifold[*lib_section].as_dynamic_symbol_table()
                            else {
                                continue;
                            };

                            // Try to find a symbol with the corresponding name, or skip if it is not found.
                            let Some((lib_sym, _)) = lib_section
                                .symbol_iter(manifold)
                                .filter_map(Result::ok)
                                .find(|(_, sym)| *sym == name)
                            else {
                                continue;
                            };

                            // Locates the section containing the symbol.
                            let container = &manifold[lib_obj.sections[lib_sym.st_shndx as usize]];

                            let start =
                                lib_sym.st_value as usize + container.offset - container.addr;

                            let lib_content = container.mapping.bytes().as_ptr() as usize;

                            unsafe {
                                core::ptr::copy_nonoverlapping(
                                    (lib_content + start) as *const u8,
                                    addr,
                                    lib_sym.st_size as usize,
                                );
                                break 'find_symbol;
                            }
                        }
                    }
                }
                R_X86_64_JUMP_SLOT => {
                    apply_reloc!(addr, s, u64);
                }
                R_X86_64_RELATIVE => {
                    apply_reloc!(addr, (b + a), u64);
                }
                _ => panic!("unknown rela type {:x}", r#type),
            };
        }

        Ok(())
    }
}
