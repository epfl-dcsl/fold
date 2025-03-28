use alloc::boxed::Box;
use core::slice;

use goblin::elf::reloc::{R_X86_64_64, R_X86_64_COPY, R_X86_64_JUMP_SLOT, R_X86_64_RELATIVE};
use goblin::elf64::reloc::Rela;
use log::info;

use crate::elf::ElfItemIterator;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::object::section::SectionT;
use crate::Handle;

macro_rules! apply_reloc {
    ($addr:expr, $value:expr, $size:expr) => {
        let a = $addr;
        let v = $value;
        let s = $size;

        info!("Relocating at {a:x?} with value {v:x?} ({s}B)");

        unsafe {
            slice::from_raw_parts_mut(a, s).copy_from_slice(&v.to_le_bytes());
        }
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
        let section = manifold.sections.get(section_handle).unwrap();
        log::info!(
            "Process relocation of section {:?}...",
            section.get_display_name(manifold).unwrap_or_default()
        );

        let obj = manifold.objects.get(section.obj).unwrap();
        let base = obj.pie_load_offset.unwrap() as *mut u8;

        for rela in ElfItemIterator::<Rela>::from_section(section) {
            let addr = unsafe { base.add(rela.r_offset as usize) };
            let r#type = rela.r_info as u32;
            let sym = (rela.r_info >> 32) as u32;

            match r#type {
                R_X86_64_64 => {
                    let dynsym_entry = manifold
                        .get_section_link(section)
                        .unwrap()
                        .as_dynamic_symbol_table()?
                        .get_entry(sym as usize)?;

                    apply_reloc!(
                        addr,
                        (base as i64 + rela.r_addend + dynsym_entry.st_value as i64) as u64,
                        8
                    );
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
                            let Ok(lib_section) = manifold
                                .sections
                                .get(*lib_section)
                                .unwrap()
                                .as_dynamic_symbol_table()
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
                            let container = manifold
                                .sections
                                .get(lib_obj.sections[lib_sym.st_shndx as usize])
                                .unwrap();

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
                    let dynsym_entry = manifold
                        .get_section_link(section)
                        .unwrap()
                        .as_dynamic_symbol_table()?
                        .get_entry(sym as usize)?;

                    apply_reloc!(addr, (base as i64 + dynsym_entry.st_value as i64) as u64, 8);
                }
                R_X86_64_RELATIVE => {
                    apply_reloc!(addr, (base as i64 + rela.r_addend) as u64, 8);
                }
                _ => panic!("unknown rela type {:x}", r#type),
            };
        }

        Ok(())
    }
}
