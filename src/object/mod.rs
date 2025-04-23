use alloc::borrow::ToOwned;
use alloc::ffi::CString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::CStr;

use goblin::elf::section_header::SHN_UNDEF;
use goblin::elf::sym::{STB_GLOBAL, STB_LOCAL, STB_WEAK};
use goblin::elf64::sym::Sym;

use crate::arena::Handle;
use crate::elf::{cst, sym_bindings, ElfHeader, ElfItemIterator, ProgramHeader, SectionHeader};
use crate::error::FoldError;
use crate::exit::exit_error;
use crate::file::{Mapping, MappingMut};
use crate::filters::ObjectFilter;
use crate::manifold::Manifold;
use crate::share_map::ShareMap;
use crate::{Section, SymbolTableSection};

pub mod section;

// ———————————————————————————————— Objects ————————————————————————————————— //

/// An elf object.
pub struct Object {
    path: CString,
    mapping: Arc<Mapping>,

    pub(crate) sections: Vec<Handle<Section>>,
    pub(crate) segments: Vec<Handle<Segment>>,

    /// OS ABI
    os_abi: u8,
    /// Elf type
    elf_type: u16,
    /// Offset of the section header table.
    e_shoff: usize,
    /// Size of the entries in the section header table.
    e_shentsize: u16,
    /// Number of entries in the section header table.
    pub(crate) e_shnum: u16,
    /// Offset of the program header table.
    e_phoff: usize,
    /// Size of entries in the program header table.
    e_phentsize: u16,
    /// Number of entries in the program header table.
    pub(crate) e_phnum: u16,
    /// Index of the section header string table
    e_shstrndx: u16,

    pub shared: ShareMap,

    pub is_lib: bool,
}

impl Object {
    pub fn new(file: Arc<Mapping>, path: CString) -> Self {
        let hdr = as_header(file.bytes());
        let obj = Self {
            // Completed by the Manifold
            sections: Vec::new(),
            // Completed by the Manifold
            segments: Vec::new(),
            path,
            os_abi: hdr.e_ident[0],
            elf_type: hdr.e_type,
            e_shoff: hdr.e_shoff as usize,
            e_shentsize: hdr.e_shentsize,
            e_shnum: hdr.e_shnum,
            e_phoff: hdr.e_phoff as usize,
            e_phentsize: hdr.e_phentsize,
            e_phnum: hdr.e_phnum,
            e_shstrndx: hdr.e_shstrndx,
            mapping: file,
            shared: ShareMap::new(),
            is_lib: false
        };

        if let Err(err) = obj.validate() {
            let path = obj.path.to_str().unwrap_or("<path is not utf-8>");
            log::error!("{} for file {}", err, path);
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
        if ident[cst::EI_VERSION] != 1 {
            return Err("Invalid elf version");
        }

        Ok(())
    }

    pub(crate) fn matches(&self, filter: ObjectFilter) -> bool {
        match filter.mask {
            crate::filters::ObjectMask::Strict => {
                filter.elf_type == self.elf_type && filter.os_abi == self.os_abi
            }
            crate::filters::ObjectMask::ElfType => filter.elf_type == self.elf_type,
            crate::filters::ObjectMask::OsAbi => filter.os_abi == self.os_abi,
            crate::filters::ObjectMask::Any => true,
        }
    }

    pub fn raw_slice(&self, offset: usize, len: usize) -> &[u8] {
        &self.mapping.bytes()[offset..(offset + len)]
    }

    pub fn header(&self) -> &ElfHeader {
        as_header(self.raw())
    }

    pub fn display_path(&self) -> &str {
        match self.path.to_str() {
            Ok(path) => path,
            Err(_) => "<path is not utf-8>",
        }
    }

    pub fn raw(&self) -> &[u8] {
        self.mapping.bytes()
    }

    pub fn section_headers(&self) -> ElfItemIterator<SectionHeader> {
        ElfItemIterator::new(self.raw(), self.e_shoff, self.e_shnum, self.e_shentsize)
    }

    pub fn program_headers(&self) -> ElfItemIterator<ProgramHeader> {
        ElfItemIterator::new(self.raw(), self.e_phoff, self.e_phnum, self.e_phentsize)
    }

    pub fn find_symbol_<'a>(
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
            // List all entries with matching symbol name
            let matching_entries: Vec<(&Sym, &CStr)> = section
                .symbol_iter(manifold)
                .filter_map(Result::ok)
                .filter(|(_, name)| *name == symbol)
                .collect::<Vec<_>>();

            // Find an entry with LOCAL or GLOBAL visibility. If none match, use the first entry found (thus including
            // WEAK entries as well). This implements priority between LOCAL/GLOBAL and WEAK.
            let entry: Option<(&Sym, &CStr)> = matching_entries
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

                    // Section containing the symbol
                    let container = manifold
                        .sections
                        .get(self.sections[sym.st_shndx as usize])
                        .expect("Symbol not contained in a section");

                    if sym.st_info & STB_WEAK == 0 {
                        return Ok((container, *sym));
                    }

                    weak_result = Ok((container, *sym))
                }
            }
        }

        weak_result
    }

    /// Find the given symbol in one of the [`SHT_STRTAB`](goblin::elf::section_header::SHT_STRTAB) section of this object.
    /// Symbols with binding [`STB_LOCAL`] or [`STB_GLOBAL`] take priority over [`STB_WEAK`].
    pub fn find_symbol<'a>(
        &'a self,
        symbol: &'_ CStr,
        manifold: &'a Manifold,
    ) -> Result<(&'a Section, Sym), FoldError> {
        self.find_symbol_(symbol, manifold, |s| s.as_symbol_table())
    }

    /// Find the given symbol in one of the [`SHT_DYNSYM`](goblin::elf::section_header::SHT_DYNSYM) section of this object.
    /// Symbols with binding [`STB_LOCAL`] or [`STB_GLOBAL`] take priority over [`STB_WEAK`].
    pub fn find_dynamic_symbol<'a>(
        &'a self,
        symbol: &'_ CStr,
        manifold: &'a Manifold,
    ) -> Result<(&'a Section, Sym), FoldError> {
        self.find_symbol_(symbol, manifold, |s| s.as_dynamic_symbol_table())
    }
}

// ———————————————————————————————— Segments ———————————————————————————————— //

pub struct Segment {
    /// The mapping backing this segment.
    pub mapping: Arc<Mapping>,
    /// If the segment is loadable, this is the mapping to the loaded
    pub loaded_mapping: Option<Arc<MappingMut>>,
    /// The object containing this segment.
    pub obj: Handle<Object>,
    /// The type of the program header (ph_type).
    pub tag: u32,
    /// Segment flags.
    pub flags: u32,
    /// Offset of the segment in the file.
    pub offset: usize,
    /// Virtual address of the segment.
    pub vaddr: usize,
    /// Physical address of the segment.
    pub paddr: usize,
    /// Size of the segment in the file.
    pub file_size: usize,
    /// Size of the segment in memory.
    pub mem_size: usize,
    /// Required alignment for the section.
    pub align: usize,
}

impl Segment {
    pub(crate) fn new(
        header: &ProgramHeader,
        obj_idx: Handle<Object>,
        manifold: &Manifold,
    ) -> Self {
        let obj = &manifold[obj_idx];
        let mapping = &obj.mapping;

        Self {
            mapping: mapping.clone(),
            loaded_mapping: None,
            obj: obj_idx,
            tag: header.p_type,
            flags: header.p_flags,
            offset: header.p_offset as usize,
            vaddr: header.p_vaddr as usize,
            paddr: header.p_paddr as usize,
            file_size: header.p_filesz as usize,
            mem_size: header.p_memsz as usize,
            align: header.p_align as usize,
        }
    }
}

// ———————————————————————————————— Helpers ————————————————————————————————— //

fn as_header(bytes: &[u8]) -> &ElfHeader {
    const HEADER_SIZE: usize = core::mem::size_of::<ElfHeader>();
    let bytes = bytes[0..HEADER_SIZE].try_into().unwrap();
    ElfHeader::from_bytes(bytes)
}
