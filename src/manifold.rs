use crate::arena::Arena;
use core::fmt;

// ———————————————————————————————— Manifold ———————————————————————————————— //

/// The manifold is an intermediate representation of all objects composing a program.
pub struct Manifold {
    objects: Arena<()>,
    sections: Arena<()>,
    segments: Arena<()>,
    regions: Arena<()>,
}

impl Manifold {
    pub(crate) fn new() -> Self {
        Self {
            objects: Arena::new(),
            sections: Arena::new(),
            segments: Arena::new(),
            regions: Arena::new(),
        }
    }
}

// ———————————————————————————— Manifold Item ID ———————————————————————————— //

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ItemFilter {
    Object(ObjectFilter),
    Segment(SegmentID, ObjectFilter),
    Section(SectionID, ObjectFilter),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ObjectFilter {
    /// A mask to ignore filters on OS ABIs or elf types.
    mask: ObjectMask,
    /// OS ABI
    os_abi: u8,
    /// Elf type
    elf_type: u16,
}

/// A mask to filter matching objects.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ObjectMask {
    /// Accept only objects matching exactly the OS ABI and elf type.
    Strict,
    /// Accept any object matching the elf type.
    ElfType,
    /// Accept any object matching the OS abi.
    OsAbi,
    /// Accept any object.
    Any,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SegmentID {
    /// Segment type, correspond to ph_type.
    segment_type: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SectionID {
    /// Sectionn type, correspond to sh_type.
    section_type: u32,
}

// ———————————————————————————————— Helpers ————————————————————————————————— //

pub fn segment(segment_type: u32) -> SegmentID {
    segment_type.into()
}

pub fn section(section_type: u32) -> SectionID {
    section_type.into()
}

impl From<u32> for SegmentID {
    fn from(segment_type: u32) -> Self {
        Self { segment_type }
    }
}

impl From<u32> for SectionID {
    fn from(section_type: u32) -> Self {
        Self { section_type }
    }
}

impl From<ObjectFilter> for ItemFilter {
    fn from(object: ObjectFilter) -> Self {
        ItemFilter::Object(object)
    }
}

impl From<SegmentID> for ItemFilter {
    fn from(segment: SegmentID) -> Self {
        ItemFilter::Segment(segment, ObjectFilter::any())
    }
}

impl From<SectionID> for ItemFilter {
    fn from(section: SectionID) -> Self {
        ItemFilter::Section(section, ObjectFilter::any())
    }
}

impl ObjectFilter {
    pub fn any() -> Self {
        Self {
            mask: ObjectMask::Any,
            os_abi: 0,
            elf_type: 0,
        }
    }
}

// ———————————————————————————————— Display ————————————————————————————————— //

impl fmt::Debug for ItemFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Object(obj) => write!(f, "[{obj:?}]"),
            Self::Segment(seg, obj) => write!(f, "[{seg:?}, {obj:?}]"),
            Self::Section(sec, obj) => write!(f, "[{sec:?}, {obj:?}]"),
        }
    }
}

impl fmt::Debug for ObjectFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.mask {
            ObjectMask::Strict => write!(
                f,
                "Object(abi: 0x{:x}, type: 0x{:x})",
                self.os_abi, self.elf_type
            ),
            ObjectMask::ElfType => write!(f, "Object(type: 0x{:x})", self.elf_type),
            ObjectMask::OsAbi => write!(f, "Object(abi: 0x{:x})", self.os_abi),
            ObjectMask::Any => write!(f, "Object(any)"),
        }
    }
}

impl fmt::Debug for SegmentID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Segment(0x{:x})", self.segment_type)
    }
}

impl fmt::Debug for SectionID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Section(0x{:x})", self.section_type)
    }
}
