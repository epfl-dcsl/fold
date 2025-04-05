use alloc::boxed::Box;
use alloc::ffi::CString;
use core::str::FromStr;

use goblin::elf::reloc::{R_X86_64_64, R_X86_64_COPY, R_X86_64_JUMP_SLOT, R_X86_64_RELATIVE, *};
use goblin::elf::section_header::SHN_UNDEF;
use goblin::elf::sym::{st_type, STB_GLOBAL, STB_WEAK};
use goblin::elf64::reloc::{self, Rela};

use crate::elf::ElfItemIterator;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::object::section::SectionT;
use crate::sysv::error::SysvError;
use crate::Handle;

macro_rules! apply_reloc {
    ($addr:expr, $value:expr, $type:ty) => {
        let value = $value;
        // log::trace!("Relocate {:x?} to 0x{:x?}", $addr, value);
        unsafe { core::ptr::write_unaligned($addr as *mut $type, value as $type) };
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
            .load_offset
            .ok_or(SysvError::RelaSectionWithoutVirtualAdresses)? as *mut u8;

        let b = base as i64;
        let _got: i64 = obj
            .find_symbol(
                CString::from_str("_GLOBAL_OFFSET_TABLE_")
                    .unwrap()
                    .as_c_str(),
                manifold,
            )
            .map(|entry| b + entry.1.st_value as i64)
            .unwrap_or_default();

        'rela: for rela in ElfItemIterator::<Rela>::from_section(section) {
            let addr = unsafe { base.add(rela.r_offset as usize) };
            let r#type = reloc::r_type(rela.r_info);
            let sym = reloc::r_sym(rela.r_info);

            let a = rela.r_addend;
            let s: Result<i64, crate::error::FoldError> = section
                .get_linked_section(manifold)?
                .as_dynamic_symbol_table()?
                .get_symbol_and_entry(sym as usize, &manifold)
                .map(|(name, entry)| {
                    if entry.st_shndx as u32 == SHN_UNDEF && st_type(entry.st_info) == STB_GLOBAL {
                        let o = manifold
                            .objects
                            .enumerate()
                            .filter(|o| o.0 != section.obj)
                            .find_map(|o| o.1.find_symbol(name, manifold).ok())
                            .unwrap();
                        manifold[o.0.obj].load_offset.unwrap() as i64 + o.1.st_value as i64
                    } else {
                        b + entry.st_value as i64
                    }
                });

            // See https://web.archive.org/web/20250319095707/https://gitlab.com/x86-psABIs/x86-64-ABI
            match r#type {
                R_X86_64_NONE => {}
                R_X86_64_64 => {
                    apply_reloc!(addr, s? + a, u64);
                }
                R_X86_64_COPY => {
                    let name = section
                        .get_linked_section(manifold)?
                        .as_dynamic_symbol_table()?
                        .get_symbol(sym as usize, manifold)?;

                    'find_symbol: for (_, lib_obj) in
                        manifold.objects.enumerate().filter(|s| s.0 != section.obj)
                    {
                        let Ok((_, lib_sym)) = lib_obj.find_symbol(name, manifold) else {
                            continue;
                        };

                        // Locates the section containing the symbol.
                        let container = &manifold[lib_obj.sections[lib_sym.st_shndx as usize]];

                        let start = lib_sym.st_value as usize + container.offset - container.addr;

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
                R_X86_64_JUMP_SLOT => {
                    apply_reloc!(addr, s?, u64);
                }
                R_X86_64_GLOB_DAT => {
                    if (rela.r_info & (1 << STB_WEAK as u64)) != 0 {
                        apply_reloc!(addr, s?, u64);
                    }

                    let name = section
                        .get_linked_section(manifold)?
                        .as_dynamic_symbol_table()?
                        .get_symbol(sym as usize, manifold)?;

                    for (_, lib_obj) in manifold.objects.enumerate() {
                        let Ok((_, lib_sym)) = lib_obj.find_dynamic_symbol(name, manifold) else {
                            continue;
                        };
                        if lib_sym.st_value == 0 {
                            continue;
                        }

                        // Locates the section containing the symbol.
                        let container = &manifold[lib_obj.sections[lib_sym.st_shndx as usize]];
                        let start = lib_sym.st_value as usize + container.offset - container.addr;

                        let lib_content = lib_obj.load_offset.unwrap_or_default();

                        apply_reloc!(addr, lib_content + start, u64);
                        continue 'rela;
                    }
                }
                R_X86_64_32 | R_X86_64_32S => {
                    apply_reloc!(addr, s? + a, u32);
                }
                R_X86_64_16 => {
                    apply_reloc!(addr, s? + a, u16);
                }
                R_X86_64_8 => {
                    apply_reloc!(addr, s? + a, u8);
                }
                R_X86_64_RELATIVE => {
                    apply_reloc!(addr, b + a, u64);
                }
                R_X86_64_DTPMOD64 | R_X86_64_DTPOFF64 | R_X86_64_TPOFF64 | R_X86_64_TLSGD
                | R_X86_64_TLSLD | R_X86_64_DTPOFF32 | R_X86_64_GOTTPOFF | R_X86_64_TPOFF32 => {
                    /* Used for Thread Local Storage */
                }
                R_X86_64_IRELATIVE => {
                    let code: extern "C" fn() -> i64 = unsafe { core::mem::transmute(b + a) };
                    apply_reloc!(addr, code(), i64);
                }
                _ => panic!("unknown rela type 0x{:x}", r#type),
            };
        }

        Ok(())
    }
}
