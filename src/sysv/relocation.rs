use crate::elf::ElfItemIterator;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::object::section::SectionT;
use crate::sysv::error::SysvError;
use crate::Handle;
use alloc::boxed::Box;
use goblin::elf::reloc::*;
use goblin::elf64::reloc::Rela;

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
        let obj = &manifold.objects[section.obj];

        log::info!(
            "Process relocation of section {:?} for object {}...",
            section.get_display_name(manifold).unwrap_or_default(),
            obj.display_path()
        );

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

            let got = obj
                .sections
                .iter()
                .map(|h| &manifold.sections[*h])
                .filter_map(|s| s.as_dynamic_symbol_table().ok())
                .filter_map(|s| {
                    s.symbol_iter(&manifold)
                        .filter_map(Result::ok)
                        .find(|(_, name)| name.to_str().is_ok_and(|n| n == "_GLOBAL_OFFSET_TABLE_"))
                        .iter()
                        .next()
                        .map(|(entry, _)| (*entry).clone())
                })
                .next();
            let g = if let Some(got) = got {
                b + got.st_value as i64
            } else {
                0
            };

            // See https://web.archive.org/web/20250319095707/https://gitlab.com/x86-psABIs/x86-64-ABI
            match r#type {
                R_X86_64_NONE => {}
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
                R_X86_64_JUMP_SLOT | R_X86_64_GLOB_DAT => {
                    apply_reloc!(addr, s, u64);
                }
                R_X86_64_32 | R_X86_64_32S => {
                    apply_reloc!(addr, (s + a), u32);
                }
                R_X86_64_16 => {
                    apply_reloc!(addr, (s + a), u16);
                }
                R_X86_64_8 => {
                    apply_reloc!(addr, (s + a), u8);
                }
                R_X86_64_RELATIVE => {
                    apply_reloc!(addr, (b + a), u64);
                }
                _ => panic!("unknown rela type 0x{:x}", r#type),
            };
        }

        Ok(())
    }
}
