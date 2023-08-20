use alloc::ffi::CString;
use alloc::sync::Arc;

use crate::arena::Handle;
use crate::elf::{cst, ElfHeader, ElfItemIterator, ProgramHeader, SectionHeader};
use crate::exit::exit_error;
use crate::file::Mapping;
use crate::manifold::Manifold;

// ———————————————————————————————— Objects ————————————————————————————————— //

/// An elf object.
pub struct Object {
    path: CString,
    mapping: Arc<Mapping>,

    /// Offset of the section header table.
    e_shoff: usize,
    /// Size of the entries in the section header table.
    e_shentsize: u16,
    /// Number of entries in the section header table.
    e_shnum: u16,
    /// Offset of the program header table.
    e_phoff: usize,
    /// Size of entries in the program header table.
    e_phentsize: u16,
    /// Number of entries in the program header table.
    e_phnum: u16,
}

impl Object {
    pub fn new(file: Arc<Mapping>, path: CString) -> Self {
        let hdr = as_header(file.bytes());
        let obj = Self {
            path,
            e_shoff: hdr.e_shoff as usize,
            e_shentsize: hdr.e_shentsize,
            e_shnum: hdr.e_shnum,
            e_phoff: hdr.e_phoff as usize,
            e_phentsize: hdr.e_phentsize,
            e_phnum: hdr.e_phnum,
            mapping: file,
        };

        if let Err(err) = obj.validate() {
            let path = match obj.path.to_str() {
                Ok(s) => s,
                Err(_) => "<path is not utf-8>",
            };
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

    pub fn raw_slice(&self, offset: usize, len: usize) -> &[u8] {
        &self.mapping.bytes()[offset..(offset + len)]
    }

    pub fn header(&self) -> &ElfHeader {
        as_header(&self.raw())
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

// ———————————————————————————————— Sections ———————————————————————————————— //

pub struct Section {
    mapping: Arc<Mapping>,
    /// Offset to the name of the section in the object's .shstrtab section.
    /// TODO: store name directly.
    name: u32,
    /// The type of the section (sh_type).
    tag: u32,
    /// Section flags.
    flags: usize,
    /// Virtual address once loaded, for loadable sections.
    addr: usize,
    /// Offset of the section in the file.
    offset: usize,
    /// Size of the section in the file.
    size: usize,
    /// Required alignment.
    alig: usize,
    /// Link to an associated section.
    /// TODO: store a handle instead
    link: u32,
    /// Extra information about the section.
    info: u32,
    /// Size of the elements contained in the section, if applicable.
    entity_size: usize,
}

impl Section {
    pub fn new(header: &SectionHeader, obj: Handle<Object>, manifold: &Manifold) -> Self {
        let obj = &manifold[obj];
        let mapping = &obj.mapping;

        // TODO: check alignment:
        // - Must be a power of 2
        // - Must be respected in the file
        let _addr_align = header.sh_addralign;

        Self {
            mapping: mapping.clone(),
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
}

// ———————————————————————————————— Helpers ————————————————————————————————— //

fn as_header(bytes: &[u8]) -> &ElfHeader {
    const HEADER_SIZE: usize = core::mem::size_of::<ElfHeader>();
    let bytes = bytes[0..HEADER_SIZE].try_into().unwrap();
    ElfHeader::from_bytes(bytes)
}
