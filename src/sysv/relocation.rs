use core::mem;

use goblin::elf::reloc::R_X86_64_RELATIVE;

use crate::{dbg, manifold::Manifold, module::Module, println, Handle};

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

        for rela in (ElfRelaIter {
            bytes: &section.mapping.bytes()[section.offset..section.offset + section.size],
        }) {
            match rela.r#type {
                R_X86_64_RELATIVE => {
                    let base = manifold.pie_load_offset.unwrap() as *mut u64;
                    unsafe {
                        *(base.add(rela.offset as usize)) = base as u64 + rela.addend;
                    };
                }
                _ => panic!("unknown rela type {:x}", rela.r#type),
            }
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
    bytes: &'a [u8],
}

impl ElfRelaIter<'_> {
    fn read_u32(&mut self) -> Option<u32> {
        let (int_bytes, rest) = self.bytes.split_at_checked(mem::size_of::<u32>())?;
        self.bytes = rest;
        TryInto::<[u8; 4]>::try_into(int_bytes)
            .ok()
            .map(u32::from_le_bytes)
    }

    fn read_u64(&mut self) -> Option<u64> {
        let (int_bytes, rest) = self.bytes.split_at(mem::size_of::<u64>());
        self.bytes = rest;
        Some(u64::from_le_bytes(int_bytes.try_into().unwrap()))
    }
}

impl Iterator for ElfRelaIter<'_> {
    type Item = Rela;

    fn next(&mut self) -> Option<Self::Item> {
        Some(Rela {
            offset: self.read_u64()?,
            r#type: self.read_u32()?,
            sym: self.read_u32()?,
            addend: self.read_u64()?,
        })
    }
}
