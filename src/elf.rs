//! # Elf
//!
//! Re-export goblin's data structures with saner paths, and offers some elf utilities.

use core::iter::Iterator;
use core::marker::PhantomData;

// ——————————————————————————————— Re-Exports ——————————————————————————————— //
pub use goblin::elf64::dynamic::Dyn;
pub use goblin::elf64::header::header64::Header as ElfHeader;
pub use goblin::elf64::program_header::ProgramHeader;
pub use goblin::elf64::reloc;
pub use goblin::elf64::reloc::{Rel, Rela};
pub use goblin::elf64::section_header::SectionHeader;
pub use goblin::elf64::sym::Sym;
use plain::Plain;

use crate::Section;

/// Re-export most of Goblin constants
pub mod cst {
    pub use goblin::elf64::dynamic::*;
    pub use goblin::elf64::header::*;
    pub use goblin::elf64::program_header::*;
    pub use goblin::elf64::section_header::*;
    pub use goblin::elf64::sym::*;

    pub const SHT_RELR: u32 = 19;
}

// —————————————————————————————— Tagged Items —————————————————————————————— //

pub trait Tagged<T> {
    fn tag(&self) -> T;
}

impl Tagged<u64> for Dyn {
    fn tag(&self) -> u64 {
        self.d_tag
    }
}

impl Tagged<u32> for ProgramHeader {
    fn tag(&self) -> u32 {
        self.p_type
    }
}

impl Tagged<u32> for SectionHeader {
    fn tag(&self) -> u32 {
        self.sh_type
    }
}

// ——————————————————————————————— Iterators ———————————————————————————————— //

pub struct ElfItemIterator<'a, T> {
    raw: &'a [u8],
    idx: usize,
    end: usize,
    item_size: usize,
    _marker: PhantomData<T>,
}

impl<'a, T> ElfItemIterator<'a, T> {
    pub fn new(raw: &'a [u8], e_shoff: usize, e_shnum: u16, e_shentsize: u16) -> Self {
        Self {
            raw,
            idx: e_shoff,
            end: e_shoff + (e_shnum as usize * e_shentsize as usize),
            item_size: e_shentsize as usize,
            _marker: PhantomData,
        }
    }

    pub fn with_len(raw: &'a [u8], offset: usize, len: usize) -> Self {
        // TODO: check that alignment & size alllows to pack them.
        let size = core::mem::size_of::<T>();
        Self {
            raw,
            idx: offset,
            end: offset + len,
            item_size: size,
            _marker: PhantomData,
        }
    }

    pub fn from_section_header(raw: &'a [u8], sh: &SectionHeader) -> Self {
        assert_eq!(sh.sh_entsize as usize, core::mem::size_of::<T>());
        Self {
            raw,
            idx: sh.sh_offset as usize,
            end: (sh.sh_offset + sh.sh_size) as usize,
            item_size: sh.sh_entsize as usize,
            _marker: PhantomData,
        }
    }

    pub fn from_section(sh: &'a Section) -> Self {
        assert_eq!(sh.entity_size as usize, core::mem::size_of::<T>());
        Self {
            raw: sh.mapping.bytes(),
            idx: sh.offset as usize,
            end: (sh.offset + sh.size) as usize,
            item_size: sh.entity_size as usize,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> Iterator for ElfItemIterator<'a, T>
where
    T: Plain + 'a,
{
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        if idx >= self.end {
            return None;
        }

        let bytes = &self.raw[idx..(idx + self.item_size)];
        let item = T::from_bytes(bytes).unwrap();
        self.idx += self.item_size;
        Some(item)
    }
}

impl<T> Clone for ElfItemIterator<'_, T> {
    fn clone(&self) -> Self {
        Self {
            raw: self.raw,
            idx: self.idx,
            end: self.end,
            item_size: self.item_size,
            _marker: PhantomData,
        }
    }
}

// ———————————————————————————————— Helpers ————————————————————————————————— //

impl<'a, T> ElfItemIterator<'a, T>
where
    T: Plain + 'a,
{
    pub fn find_tag<Tag>(&self, tag: Tag) -> Option<&'a T>
    where
        T: Tagged<Tag>,
        Tag: Eq,
    {
        self.clone().find(|x| x.tag() == tag)
    }
}

impl<'a> ElfItemIterator<'a, ProgramHeader> {
    pub fn find_segment<'b>(&'b mut self, p_type: u32) -> Option<&'a ProgramHeader> {
        self.find(|segment| segment.p_type == p_type)
    }
}

impl<'a> ElfItemIterator<'a, SectionHeader> {
    pub fn find_section<'b>(&'b mut self, sh_type: u32) -> Option<&'a SectionHeader> {
        self.find(|section| section.sh_type == sh_type)
    }
}
