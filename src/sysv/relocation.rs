use core::slice;

use goblin::elf::reloc::R_X86_64_RELATIVE;
use goblin::elf64::reloc::Rela;
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

            match r#type {
                R_X86_64_RELATIVE => {
                    apply_reloc!(addr, (base as i64 + rela.r_addend) as u64, 8);
                }
                _ => panic!("unknown rela type {:x}", r#type),
            };
        }
    }
}
