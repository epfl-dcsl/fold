use alloc::sync::Arc;

use goblin::elf64::program_header::ProgramHeader;

use crate::arena::Handle;
use crate::elf::Object;
use crate::file::{Mapping, MappingMut};
use crate::{Manifold, ShareMap};

pub struct Segment {
    /// The mapping representing this segment in the object.
    pub mapping: Mapping,
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

    /// Shared memory specific to this object.
    pub shared: ShareMap,
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
            mapping: Mapping {
                bytes: &mapping.bytes()
                    [header.p_offset as usize..(header.p_offset + header.p_filesz) as usize],
                fd: None,
            },
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
            shared: ShareMap::new(),
        }
    }
}
