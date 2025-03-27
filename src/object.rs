use alloc::ffi::CString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::CStr;

use goblin::elf::section_header::{SHT_DYNSYM, SHT_STRTAB};

use crate::arena::Handle;
use crate::elf::{cst, ElfHeader, ElfItemIterator, ProgramHeader, SectionHeader};
use crate::error::FoldError;
use crate::exit::exit_error;
use crate::file::{Mapping, MappingMut};
use crate::filters::ObjectFilter;
use crate::manifold::Manifold;

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

    pub pie_load_offset: Option<usize>,
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
            mapping: file,
            pie_load_offset: None,
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
}

// ———————————————————————————————— Segments ———————————————————————————————— //

pub struct Segment {
    /// The mapping backing this segment.
    pub mapping: Arc<Mapping>,
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

// ———————————————————————————————— Sections ———————————————————————————————— //

pub struct Section {
    /// The mapping backing this section.
    pub mapping: Arc<Mapping>,
    /// The object containing this section.
    pub obj: Handle<Object>,
    /// Offset to the name of the section in the object's .shstrtab section.
    /// TODO: store name directly.
    pub name: u32,
    /// The type of the section (sh_type).
    pub tag: u32,
    /// Section flags.
    pub flags: usize,
    /// Virtual address once loaded, for loadable sections.
    pub addr: usize,
    /// Offset of the section in the file.
    pub offset: usize,
    /// Size of the section in the file.
    pub size: usize,
    /// Required alignment.
    pub alig: usize,
    /// Link to an associated section.
    /// TODO: store a handle instead
    pub link: u32,
    /// Extra information about the section.
    pub info: u32,
    /// Size of the elements contained in the section, if applicable.
    pub entity_size: usize,
}

impl Section {
    pub(crate) fn new(
        header: &SectionHeader,
        obj_idx: Handle<Object>,
        manifold: &Manifold,
    ) -> Self {
        let obj = &manifold[obj_idx];
        let mapping = &obj.mapping;

        // TODO: check alignment:
        // - Must be a power of 2
        // - Must be respected in the file
        let _addr_align = header.sh_addralign;

        Self {
            mapping: mapping.clone(),
            obj: obj_idx,
            name: header.sh_name,
            tag: header.sh_type,
            flags: header.sh_flags as usize,
            addr: header.sh_addr as usize,
            offset: header.sh_offset as usize,
            size: header.sh_size as usize,
            alig: header.sh_addralign as usize,
            link: header.sh_link,
            info: header.sh_info,
            entity_size: header.sh_entsize as usize,
        }
    }

    pub fn as_string_table<'a>(&'a self) -> Result<StringTableSection<'a>, FoldError> {
        if self.tag == SHT_STRTAB {
            Ok(StringTableSection { section: self })
        } else {
            Err(FoldError::InvalidSectionCast {
                expected: SHT_STRTAB,
                actual: self.tag,
            })
        }
    }

    pub fn as_dynamic_symbol_table<'a>(&'a self) -> Result<DynamicSymbolSection<'a>, FoldError> {
        if self.tag == SHT_DYNSYM {
            Ok(DynamicSymbolSection { section: self })
        } else {
            Err(FoldError::InvalidSectionCast {
                expected: SHT_DYNSYM,
                actual: self.tag,
            })
        }
    }

    pub fn get_linked_section<'a>(
        &'_ self,
        manifold: &'a Manifold,
    ) -> Result<&'a Section, FoldError> {
        let obj = manifold.objects.get(self.obj).unwrap();

        manifold
            .sections
            .get(obj.sections[self.link as usize])
            .ok_or(FoldError::MissingLinkedSection)
    }
}

// ———————————————————————————————— StringTableSection ————————————————————————————————— //

pub struct StringTableSection<'a> {
    pub section: &'a Section,
}

impl<'a> StringTableSection<'a> {
    pub fn get_symbol(&self, index: usize) -> Result<&'a CStr, FoldError> {
        CStr::from_bytes_until_nul(&self.section.mapping.bytes()[(self.section.offset + index)..])
            .map_err(|_| FoldError::InvalidString)
    }
}

// ———————————————————————————————— DynamicSymbolSection ————————————————————————————————— //

pub struct DynamicSymbolSection<'a> {
    pub section: &'a Section,
}

impl<'a> DynamicSymbolSection<'a> {
    pub fn get_entry(&self, index: usize) -> Result<goblin::elf::sym::sym64::Sym, FoldError> {
        self.entry_iter()
            .nth(index)
            .copied()
            .ok_or(FoldError::OutOfBounds)
    }

    pub fn get_symbol(&self, index: usize, manifold: &'a Manifold) -> Result<&'a CStr, FoldError> {
        let entry = self.get_entry(index)?;

        self.section
            .get_linked_section(manifold)?
            .as_string_table()?
            .get_symbol(entry.st_name as usize)
    }

    pub fn entry_iter(&self) -> impl Iterator<Item = &goblin::elf::sym::sym64::Sym> {
        ElfItemIterator::<goblin::elf::sym::sym64::Sym>::from_section(self.section)
    }

    pub fn symbol_iter(
        &self,
        manifold: &'a Manifold,
    ) -> impl Iterator<Item = Result<(&goblin::elf::sym::sym64::Sym, &CStr), FoldError>> {
        self.entry_iter().map(|sym_entry| {
            self.section
                .get_linked_section(manifold)?
                .as_string_table()?
                .get_symbol(sym_entry.st_name as usize)
                .map(|symbol| (sym_entry, symbol))
        })
    }
}

// ———————————————————————————————— Helpers ————————————————————————————————— //

fn as_header(bytes: &[u8]) -> &ElfHeader {
    const HEADER_SIZE: usize = core::mem::size_of::<ElfHeader>();
    let bytes = bytes[0..HEADER_SIZE].try_into().unwrap();
    ElfHeader::from_bytes(bytes)
}
