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

    fn process_section(&mut self, section: Handle<crate::Section>, manifold: &mut Manifold) {
        log::info!("Process relocation...");

        let section = manifold.sections.get(section).unwrap();
        let base = manifold.pie_load_offset.unwrap() as *mut u8;

        for rela in ElfItemIterator::<Rela>::from_section(section) {
            let addr = unsafe { base.add(rela.r_offset as usize) };
            let r#type = rela.r_info as u32;
            let sym = (rela.r_info >> 32) as u32;
            log::info!("{sym} {:x}", rela.r_info);

            match r#type {
                R_X86_64_64 => {
                    apply_reloc!(addr, (base as i64 + rela.r_addend) as u64, 8);
                }
                R_X86_64_COPY => {
                    let obj = manifold.objects.get(section.obj).unwrap();
                    let dynsym = manifold
                        .sections
                        .get(obj.sections[section.link as usize])
                        .unwrap();

                    let dynsym_entry = ElfItemIterator::<Sym>::from_section(dynsym)
                        .nth(sym as usize)
                        .unwrap();

                    let strtab = manifold
                        .sections
                        .get(obj.sections[dynsym.link as usize])
                        .unwrap();

                    let b: &[u8] = strtab.mapping.bytes();
                    let name = CStr::from_bytes_until_nul(
                        &b[(dynsym_entry.st_name as usize + strtab.offset)..],
                    )
                    .unwrap();

                    rela.r_info as u32;

                    for (handle, lib_obj) in manifold.objects.enumerate() {
                        if handle != section.obj {
                            for lib_section in &lib_obj.sections {
                                let lib_section = manifold.sections.get(*lib_section).unwrap();

                                if lib_section.tag == SHT_DYNSYM {
                                    let lib_strtab = manifold
                                        .sections
                                        .get(lib_obj.sections[lib_section.link as usize])
                                        .unwrap();
                                    for lib_sym in ElfItemIterator::<Sym>::from_section(lib_section)
                                    {
                                        let b: &[u8] = lib_strtab.mapping.bytes();
                                        let lib_name = CStr::from_bytes_until_nul(
                                            &b[(lib_sym.st_name as usize + lib_strtab.offset)..],
                                        )
                                        .unwrap();
                                        if lib_name == name {
                                            let container = manifold
                                                .sections
                                                .get(lib_obj.sections[lib_sym.st_shndx as usize])
                                                .unwrap();

                                            let start = lib_sym.st_value as usize
                                                + container.offset
                                                - container.addr;

                                            let lib_content =
                                                container.mapping.bytes().as_ptr() as usize;

                                            unsafe {
                                                core::ptr::copy_nonoverlapping(
                                                    (lib_content + start) as *const u8,
                                                    addr as *mut u8,
                                                    lib_sym.st_size as usize,
                                                );
                                            }
                                        }
                                    }
                                }
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
