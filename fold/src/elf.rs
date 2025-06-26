//! # Elf utilities and constants.
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

pub use crate::object::*;

// ——————————————————————————————— Iterators ———————————————————————————————— //

/// Utility iterator that allows to parse and iterate over the content of a section.
pub struct ElfItemIterator<'a, T> {
    raw: &'a [u8],
    idx: usize,
    end: usize,
    item_size: usize,
    _marker: PhantomData<T>,
}

impl<'a, T> ElfItemIterator<'a, T> {
    /// Creates an iterator from raw values.
    ///
    /// - `raw`: The content of the object containing the section.
    /// - `e_shoff`: The position of the section in the object.
    /// - `e_shnum`: The number of entries in the section.
    /// - `e_shentsize`: The size of an entry. Must be equal to the size of `T`.
    pub fn new(raw: &'a [u8], e_shoff: usize, e_shnum: u16, e_shentsize: u16) -> Self {
        assert_eq!(e_shentsize as usize, core::mem::size_of::<T>());

        Self {
            raw,
            idx: e_shoff,
            end: e_shoff + (e_shnum as usize * e_shentsize as usize),
            item_size: e_shentsize as usize,
            _marker: PhantomData,
        }
    }

    /// Creates an iterator from an offset an a size.
    ///
    /// - `raw`: The content of the object containing the section.
    /// - `e_shoff`: The position of the section in the object.
    /// - `e_shsize`: The size of the section.
    pub fn with_len(raw: &'a [u8], e_shoff: usize, e_shsize: usize) -> Self {
        // TODO: check that alignment & size allows to pack them.
        let size = core::mem::size_of::<T>();
        Self {
            raw,
            idx: e_shoff,
            end: e_shoff + e_shsize,
            item_size: size,
            _marker: PhantomData,
        }
    }

    /// Creates an iterator from a section header entry.
    ///
    /// - `raw`: The content of the object containing the section.
    /// - `sh`: The corresponding section header entry.
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

    /// Creates an iterator over a section.
    pub fn from_section(sh: &'a Section) -> Self {
        assert_eq!({ sh.entity_size }, core::mem::size_of::<T>());
        Self {
            raw: sh.mapping.bytes(),
            idx: sh.offset,
            end: (sh.offset + sh.size),
            item_size: sh.entity_size,
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

// —————————————————————————— Helpers for symbols ——————————————————————————— //

/// Return the `BINDING` value of the symbol.
pub fn sym_bindings(sym: &Sym) -> u8 {
    sym.st_info >> 4
}
