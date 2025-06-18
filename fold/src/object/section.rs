use alloc::sync::Arc;
use core::ffi::CStr;

use goblin::elf::section_header::*;
use goblin::elf64::section_header::SectionHeader;

use super::Object;
use crate::elf::ElfItemIterator;
use crate::error::FoldError;
use crate::file::Mapping;
use crate::manifold::Manifold;
use crate::Handle;

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
    ($fn:ident,$struc:tt,$tag:expr,$name:literal) => {
        #[doc = concat!("Creates a wrapper around this section to use is as a ", $name, " section.")]
        pub fn $fn(&'_ self) -> Result<$struc<'_>, FoldError> {
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

/// Representation of an ELF Section in the Manifold.
///
/// All the attributes of the section can be accessed directly. For more complex manipulation, the `as_*` methods can be used to create tag-checked wrappers around the section.
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

    as_section!(
        as_string_table,
        StringTableSection,
        SHT_STRTAB,
        "string table"
    );
    as_section!(
        as_dynamic_symbol_table,
        SymbolTableSection,
        SHT_DYNSYM,
        "dynamic symbol table"
    );
    as_section!(
        as_symbol_table,
        SymbolTableSection,
        SHT_SYMTAB,
        "dynamic symbol table"
    );
}

// ———————————————————————————————— SectionTrait ————————————————————————————————— //

/// Common trait for all sections wrappers to group general functions.
pub trait SectionT {
    fn section(&self) -> &Section;

    /// Return the section which index is stored in `link`, fetching it from the Manifold.
    fn get_linked_section<'a>(&'_ self, manifold: &'a Manifold) -> Result<&'a Section, FoldError> {
        let obj = manifold.objects.get(self.section().obj).unwrap();

        manifold
            .sections
            .get(obj.sections[self.section().link as usize])
            .ok_or(FoldError::MissingLinkedSection)
    }

    /// Return the name of the section stored in the `.shstrtab` section.
    fn get_display_name<'a>(&self, manifold: &'a Manifold) -> Result<&'a CStr, FoldError> {
        let obj = manifold.objects.get(self.section().obj).unwrap();
        manifold.sections[obj.sections[obj.e_shstrndx as usize]]
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

/// Wrapper over a string table section (`STRTAB`), exposing extra methods to manipulate the table.
pub struct StringTableSection<'a> {
    pub section: &'a Section,
}
derive_sectiont!(StringTableSection<'_>);

impl<'a> StringTableSection<'a> {
    /// Returns the symbol at the given offset in the table.
    pub fn get_symbol(&self, index: usize) -> Result<&'a CStr, FoldError> {
        CStr::from_bytes_until_nul(&self.section.mapping.bytes()[(self.section.offset + index)..])
            .map_err(|_| FoldError::InvalidString)
    }
}

// ———————————————————————————————— DynamicSymbolSection ————————————————————————————————— //

/// Wrapper over a string table section (`DYNSYM`), exposing extra methods to query the symbols.
pub struct SymbolTableSection<'a> {
    pub section: &'a Section,
}
derive_sectiont!(SymbolTableSection<'_>);

impl<'a> SymbolTableSection<'a> {
    /// Return the `DYNSYM` entry at the given index.
    pub fn get_entry(&self, index: usize) -> Result<goblin::elf::sym::sym64::Sym, FoldError> {
        self.entry_iter()
            .nth(index)
            .copied()
            .ok_or(FoldError::OutOfBounds)
    }

    /// Return the symbol represented by the `DYNSYM` entry at the given index.
    pub fn get_symbol_name(
        &self,
        index: usize,
        manifold: &'a Manifold,
    ) -> Result<&'a CStr, FoldError> {
        let entry = self.get_entry(index)?;

        self.section
            .get_linked_section(manifold)?
            .as_string_table()?
            .get_symbol(entry.st_name as usize)
    }

    pub fn get_symbol_and_entry(
        &self,
        index: usize,
        manifold: &'a Manifold,
    ) -> Result<(&'a CStr, goblin::elf::sym::sym64::Sym), FoldError> {
        let entry = self.get_entry(index)?;

        let name = self
            .section
            .get_linked_section(manifold)?
            .as_string_table()?
            .get_symbol(entry.st_name as usize)?;

        Ok((name, entry))
    }

    /// Create an iterator over all the `DYNSYM` entries of the section.
    pub fn entry_iter(&self) -> impl Iterator<Item = &'a goblin::elf::sym::sym64::Sym> + 'a {
        ElfItemIterator::<goblin::elf::sym::sym64::Sym>::from_section(self.section)
    }

    /// Create an iterator over all the symbols referenced by the section, and their corresponding `DYNSYM` entries.
    pub fn symbol_iter(
        &self,
        manifold: &'a Manifold,
    ) -> impl Iterator<Item = Result<(goblin::elf::sym::sym64::Sym, &'a CStr), FoldError>> + 'a
    {
        self.entry_iter().map(|sym_entry| {
            self.section
                .get_linked_section(manifold)?
                .as_string_table()?
                .get_symbol(sym_entry.st_name as usize)
                .map(|symbol| (*sym_entry, symbol))
        })
    }
}
