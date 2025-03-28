// ———————————————————————————————— Sections ———————————————————————————————— //

use core::ffi::CStr;

use alloc::sync::Arc;
use goblin::{elf::section_header::*, elf64::section_header::SectionHeader};

use crate::{elf::ElfItemIterator, error::FoldError, file::Mapping, manifold::Manifold, Handle};

use super::Object;

macro_rules! derive_sectiont {
    ($struct:ty) => {
        impl SectionT for $struct {
            fn section(&self) -> &Section {
                self.section
            }
        }
    };
}

macro_rules! as_section {
    ($fn:ident,$struc:tt,$tag:expr) => {
        pub fn $fn(&self) -> Result<$struc, FoldError> {
            if self.tag == $tag {
                Ok($struc { section: self })
            } else {
                Err(FoldError::InvalidSectionCast {
                    expected: $tag,
                    actual: self.tag,
                })
            }
        }
    };
}

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

    as_section!(as_string_table, StringTableSection, SHT_STRTAB);
    as_section!(as_dynamic_symbol_table, DynamicSymbolSection, SHT_DYNSYM);
}

// ———————————————————————————————— SectionTrait ————————————————————————————————— //

pub trait SectionT {
    fn section(&self) -> &Section;

    fn get_linked_section<'a>(&'_ self, manifold: &'a Manifold) -> Result<&'a Section, FoldError> {
        let obj = manifold.objects.get(self.section().obj).unwrap();

        manifold
            .sections
            .get(obj.sections[self.section().link as usize])
            .ok_or(FoldError::MissingLinkedSection)
    }

    fn get_display_name<'a>(&self, manifold: &'a Manifold) -> Result<&'a CStr, FoldError> {
        let obj = manifold.objects.get(self.section().obj).unwrap();
        manifold
            .sections
            .get(obj.sections[obj.e_shstrndx as usize])
            .expect("Section not found")
            .as_string_table()?
            .get_symbol(self.section().name as usize)
    }
}

impl SectionT for Section {
    fn section(&self) -> &Section {
        self
    }
}

// ———————————————————————————————— StringTableSection ————————————————————————————————— //

pub struct StringTableSection<'a> {
    pub section: &'a Section,
}
derive_sectiont!(StringTableSection<'_>);

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
derive_sectiont!(DynamicSymbolSection<'_>);

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
