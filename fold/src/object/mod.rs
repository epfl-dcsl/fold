use alloc::borrow::ToOwned;
use alloc::ffi::CString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::CStr;

use goblin::elf::section_header::SHN_UNDEF;
use goblin::elf::sym::{STB_GLOBAL, STB_LOCAL, STB_WEAK};
use goblin::elf64::sym::Sym;

use crate::arena::Handle;
use crate::elf::{sym_bindings, ElfHeader, ElfItemIterator, ProgramHeader, SectionHeader};
use crate::error::FoldError;
use crate::exit::exit_error;
use crate::file::Mapping;
use crate::manifold::Manifold;
use crate::share_map::ShareMap;

mod section;
mod segment;

pub use section::*;
pub use segment::*;

// ———————————————————————————————— Objects ————————————————————————————————— //

/// An elf object.
pub struct Object {
    /// Path in the filesystem of object's file.
    pub path: CString,
    /// Raw content of the object.
    pub mapping: Arc<Mapping>,

    /// Handles in the manifold of the section of this object.
    pub sections: Vec<Handle<Section>>,
    /// Handles in the manifold of the segments of this object.
    pub segments: Vec<Handle<Segment>>,
    /// Handles in the manifold of the dependencies of this object. TODO: move into shared memory ?
    pub dependencies: Vec<Handle<Object>>,

    /// OS ABI
    pub os_abi: u8,
    /// Elf type
    pub elf_type: u16,
    /// ISA
    pub e_machine: u16,
    /// Offset of the section header table.
    pub e_shoff: usize,
    /// Size of the entries in the section header table.
    pub e_shentsize: u16,
    /// Number of entries in the section header table.
    pub e_shnum: u16,
    /// Offset of the program header table.
    pub e_phoff: usize,
    /// Size of entries in the program header table.
    pub e_phentsize: u16,
    /// Number of entries in the program header table.
    pub e_phnum: u16,
    /// Index of the section header string table.
    pub e_shstrndx: u16,

    /// Shared memory specific to this object.
    pub shared: ShareMap,
}

impl Object {
    /// Creates an object from a raw memory region. Initialy, `sections`, `segments` and `dependencies` are empty and
    /// must be filled manually.
    pub fn new(file: Arc<Mapping>, path: CString) -> Self {
        let hdr = as_header(file.bytes());
        let obj = Self {
            // Completed by the Manifold.
            sections: Vec::new(),
            // Completed by the Manifold.
            segments: Vec::new(),
            // To be completed by the loader implementation.
            dependencies: Vec::new(),
            path,
            os_abi: hdr.e_ident[0],
            elf_type: hdr.e_type,
            e_machine: hdr.e_machine,
            e_shoff: hdr.e_shoff as usize,
            e_shentsize: hdr.e_shentsize,
            e_shnum: hdr.e_shnum,
            e_phoff: hdr.e_phoff as usize,
            e_phentsize: hdr.e_phentsize,
            e_phnum: hdr.e_phnum,
            e_shstrndx: hdr.e_shstrndx,
            mapping: file,
            shared: ShareMap::new(),
        };

        if let Err(err) = obj.validate() {
            let path = obj.path.to_str().unwrap_or("<path is not utf-8>");
            log::error!("{err} for file {path}");
            exit_error();
        }

        obj
    }

    /// Validate an elf header or exit with and error.
    fn validate(&self) -> Result<(), &'static str> {
        let hdr = self.header();
        let ident = hdr.e_ident;
        if ident[0] != 0x7F || ident[1] != 0x45 || ident[2] != 0x4C || ident[3] != 0x46 {
            return Err("Invalid magic number");
        }
        if ident[goblin::elf::header::EI_VERSION] != 1 {
            return Err("Invalid elf version");
        }

        Ok(())
    }

    /// Returns a slice of the object's content from `offset` to `offset + len`.
    pub fn raw_slice(&self, offset: usize, len: usize) -> &[u8] {
        &self.mapping.bytes()[offset..(offset + len)]
    }

    /// Returns the ELF header of the object.
    pub fn header(&self) -> &ElfHeader {
        as_header(self.raw())
    }

    /// Returns the path of the file corresponding to the object as a utf-8 string.
    pub fn display_path(&self) -> &str {
        match self.path.to_str() {
            Ok(path) => path,
            Err(_) => "<path is not utf-8>",
        }
    }

    /// Raw content of the object.
    pub fn raw(&self) -> &[u8] {
        self.mapping.bytes()
    }

    /// Creates an iterator over the [`SectionHeader`] table. See also [`ElfItemIterator`]
    pub fn section_headers(&'_ self) -> ElfItemIterator<'_, SectionHeader> {
        ElfItemIterator::new(self.raw(), self.e_shoff, self.e_shnum, self.e_shentsize)
    }

    /// Creates an iterator over the [`ProgramHeader`] table. See also [`ElfItemIterator`]
    pub fn program_headers(&'_ self) -> ElfItemIterator<'_, ProgramHeader> {
        ElfItemIterator::new(self.raw(), self.e_phoff, self.e_phnum, self.e_phentsize)
    }

    /// Resolves a symbol in the object.
    ///
    /// - `symbol`: Name of the symbol to resolve.
    /// - `manifold`: The current manifold.
    /// - `symbol_table_mapper`: A function to convert a [`Section`] into a [`SymbolTableSection`]. Useful to select
    ///   between dynamic and non-dynamic symbol sections.
    fn find_symbol_<'a>(
        &'a self,
        symbol: &'_ CStr,
        manifold: &'a Manifold,
        symbol_table_mapper: impl Fn(&'a Section) -> Result<SymbolTableSection<'a>, FoldError>,
    ) -> Result<(&'a Section, Sym), FoldError> {
        let mut weak_result = Err(FoldError::SymbolNotFound(symbol.to_owned()));

        // Attempt to find a LOCAL symbol in the current object
        for section in self
            .sections
            .iter()
            .map(|h| &manifold.sections[*h])
            .filter_map(|s| symbol_table_mapper(s).ok())
        {
            // List all entries with matching symbol name.
            let matching_entries: Vec<(Sym, &CStr)> = section
                .symbol_iter(manifold)
                .filter_map(Result::ok)
                .filter(|(_, name)| *name == symbol)
                .collect::<Vec<_>>();

            // Find an entry with LOCAL or GLOBAL visibility. If none match, use the first entry found (thus including
            // WEAK entries as well). This implements priority between LOCAL/GLOBAL and WEAK.
            let entry: Option<(Sym, &CStr)> = matching_entries
                .iter()
                .find(|(sym, _)| {
                    let binding = sym_bindings(sym);

                    (binding == STB_LOCAL || binding == STB_GLOBAL)
                        && sym.st_shndx != SHN_UNDEF as u16
                })
                .or_else(|| matching_entries.first())
                .cloned();

            // If an non-weak entry is found, return it.
            if let Some((sym, _)) = entry {
                if sym.st_shndx != SHN_UNDEF as u16 {
                    // Section containing the symbol.
                    let container = manifold
                        .sections
                        .get(self.sections[sym.st_shndx as usize])
                        .expect("Symbol not contained in a section");

                    if sym.st_info & STB_WEAK == 0 {
                        return Ok((container, sym));
                    }

                    weak_result = Ok((container, sym))
                }
            }
        }

        weak_result
    }

    /// Returns an iterator over all the symbols in the object as well as their string representation.
    pub fn symbols<'a>(
        &'a self,
        manifold: &'a Manifold,
    ) -> impl Iterator<Item = Result<(goblin::elf64::sym::Sym, &'a CStr), FoldError>> + 'a {
        self.sections
            .iter()
            .map(|h: &'a Handle<Section>| &manifold.sections[*h])
            .filter_map(|s: &'a Section| s.as_symbol_table().ok())
            .flat_map(move |s: SymbolTableSection<'a>| s.symbol_iter(manifold))
    }

    /// Find the given symbol in one of the [`SHT_STRTAB`](goblin::elf::section_header::SHT_STRTAB) section of this
    /// object. Symbols with binding [`STB_LOCAL`] or [`STB_GLOBAL`] take priority over [`STB_WEAK`].
    pub fn find_symbol<'a>(
        &'a self,
        symbol: &'_ CStr,
        manifold: &'a Manifold,
    ) -> Result<(&'a Section, Sym), FoldError> {
        self.find_symbol_(symbol, manifold, |s| s.as_symbol_table())
    }

    /// Find the given symbol in one of the [`SHT_DYNSYM`](goblin::elf::section_header::SHT_DYNSYM) section of this
    /// object. Symbols with binding [`STB_LOCAL`] or [`STB_GLOBAL`] take priority over [`STB_WEAK`].
    pub fn find_dynamic_symbol<'a>(
        &'a self,
        symbol: &'_ CStr,
        manifold: &'a Manifold,
    ) -> Result<(&'a Section, Sym), FoldError> {
        self.find_symbol_(symbol, manifold, |s| s.as_dynamic_symbol_table())
    }
}

/// Returns a view of the given `bytes` as an [`ElfHeader`]. The `bytes` slice should have a length of at least
/// `sizeof(ElfHeader) == 64`.
fn as_header(bytes: &[u8]) -> &ElfHeader {
    const HEADER_SIZE: usize = core::mem::size_of::<ElfHeader>();
    let bytes = bytes[0..HEADER_SIZE].try_into().unwrap();
    ElfHeader::from_bytes(bytes)
}
