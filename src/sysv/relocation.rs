use core::slice;

use goblin::elf::reloc::R_X86_64_RELATIVE;
use log::info;

use crate::{bytes::BytesIter, manifold::Manifold, module::Module, println, Handle};

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

        for rela in ElfRelaIter::on_bytes(
            &section.mapping.bytes()[section.offset..section.offset + section.size],
        ) {
            let addr = unsafe { base.add(rela.offset as usize) };

            match rela.r#type {
                R_X86_64_RELATIVE => {
                    apply_reloc!(addr, base as u64 + rela.addend, 8);
                }
                _ => panic!("unknown rela type {:x}", rela.r#type),
            };
        }
    }
}

#[derive(Debug, Default)]
pub struct Rela {
    pub offset: u64,
    pub r#type: u32,
    pub sym: u32,
    pub addend: u64,
}

struct ElfRelaIter<'a> {
    bytes: BytesIter<'a>,
}

impl<'a> ElfRelaIter<'a> {
    pub fn on_bytes(bytes: &'a [u8]) -> Self {
        Self {
            bytes: BytesIter { bytes },
        }
    }
}

impl Iterator for ElfRelaIter<'_> {
    type Item = Rela;

    fn next(&mut self) -> Option<Self::Item> {
        Some(Rela {
            offset: self.bytes.read()?,
            r#type: self.bytes.read()?,
            sym: self.bytes.read()?,
            addend: self.bytes.read()?,
        })
    }
}
