use goblin::elf::reloc::R_X86_64_RELATIVE;

use crate::{bytes::BytesIter, dbg, manifold::Manifold, module::Module, println, Handle};

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
        "sysv-start"
    }

    fn process_section(&mut self, section: Handle<crate::Section>, manifold: &mut Manifold) {
        log::info!("Process relocation...");

        let section = manifold.sections.get(section).unwrap();

        for rela in ElfRelaIter::on_bytes(
            &section.mapping.bytes()[section.offset..section.offset + section.size],
        ) {
            println!("{:#x?}", rela);

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
