use core::ffi::CStr;
use core::slice;

use goblin::elf::reloc::{R_X86_64_64, R_X86_64_COPY, R_X86_64_RELATIVE};
use goblin::elf::section_header::SHT_DYNSYM;
use goblin::elf64::reloc::Rela;
use goblin::elf64::sym::Sym;
use log::info;

use crate::elf::ElfItemIterator;
use crate::manifold::Manifold;
use crate::module::Module;
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

    fn process_section(&mut self, section_handle: Handle<crate::Section>, manifold: &mut Manifold) {
        log::info!("Process relocation...");

        let section = manifold.sections.get(section_handle).unwrap();
        let obj = manifold.objects.get(section.obj).unwrap();
        let base = obj.pie_load_offset.unwrap() as *mut u8;

        for rela in ElfItemIterator::<Rela>::from_section(section) {
            let addr = unsafe { base.add(rela.r_offset as usize) };
            let r#type = rela.r_info as u32;
            let sym = (rela.r_info >> 32) as u32;

            match r#type {
                R_X86_64_64 => {
                    let dynsym = manifold.get_section_link(section).unwrap();

                    let dynsym_entry = ElfItemIterator::<Sym>::from_section(dynsym)
                        .nth(sym as usize)
                        .unwrap();

                    apply_reloc!(
                        addr,
                        (base as i64 + rela.r_addend + dynsym_entry.st_value as i64) as u64,
                        8
                    );
                }
                R_X86_64_COPY => {
                    let dynsym = manifold.get_section_link(section).unwrap();

                    let dynsym_entry = ElfItemIterator::<Sym>::from_section(dynsym)
                        .nth(sym as usize)
                        .unwrap();

                    let strtab = manifold.get_section_link(dynsym).unwrap();

                    let name = manifold
                        .read_symbol_value(strtab, dynsym_entry.st_name as usize)
                        .unwrap();

                    rela.r_info as u32;

                    for (_, lib_obj) in manifold.objects.enumerate().filter(|s| s.0 != section.obj)
                    {
                        for lib_section in &lib_obj.sections {
                            let lib_section = manifold.sections.get(*lib_section).unwrap();

                            if lib_section.tag != SHT_DYNSYM {
                                continue;
                            }
                            let lib_strtab = manifold.get_section_link(lib_section).unwrap();
                            let lib_sym = ElfItemIterator::<Sym>::from_section(lib_section)
                                .find(|lib_sym| {
                                    manifold
                                        .read_symbol_value(lib_strtab, lib_sym.st_name as usize)
                                        .unwrap()
                                        == name
                                })
                                .unwrap();

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
                                    addr as *mut u8,
                                    lib_sym.st_size as usize,
                                );
                                return;
                            }
                        }
                    }
                }
                R_X86_64_RELATIVE => {
                    apply_reloc!(addr, (base as i64 + rela.r_addend) as u64, 8);
                }
                _ => panic!("unknown rela type {:x}", r#type),
            };
        }
    }
}
