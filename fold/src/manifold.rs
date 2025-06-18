use alloc::borrow::ToOwned;
use alloc::ffi::CString;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::ffi::CStr;
use core::ops::{Index, IndexMut};

use goblin::elf::sym::{STB_GLOBAL, STB_LOCAL, STB_WEAK};
use goblin::elf64::sym::Sym;

use crate::arena::{Arena, Handle};
use crate::elf::sym_bindings;
use crate::error::FoldError;
use crate::file::Mapping;
use crate::object::{Object, Segment};
use crate::share_map::ShareMap;
use crate::{Env, Section};

// ———————————————————————————————— Manifold ———————————————————————————————— //

/// The manifold is an intermediate representation of all objects composing a program.
pub struct Manifold {
    pub objects: Arena<Object>,
    pub sections: Arena<Section>,
    pub segments: Arena<Segment>,
    pub shared: ShareMap,
    pub env: Env,
}

impl Manifold {
    pub(crate) fn new(env: Env) -> Self {
        Self {
            objects: Arena::new(),
            sections: Arena::new(),
            segments: Arena::new(),
            shared: ShareMap::new(),
            env,
        }
    }

    pub(crate) fn add_elf_file(&mut self, file: Mapping, path: CString) -> Handle<Object> {
        let file = Arc::new(file);
        let obj = Object::new(file.clone(), path);
        let obj_idx = self.objects.push(obj);
        let obj = &self.objects[obj_idx];

        let mut segments = Vec::with_capacity(obj.e_phnum as usize);
        for segment in obj.program_headers() {
            let segment = Segment::new(segment, obj_idx, self);
            let idx = self.segments.push(segment);
            segments.push(idx);
        }

        let mut sections = Vec::with_capacity(obj.e_shnum as usize);
        for section in obj.section_headers() {
            let section = Section::new(section, obj_idx, self);
            let idx = self.sections.push(section);
            sections.push(idx);
        }

        // Initialize segment and section indexes.
        let obj = &mut self.objects[obj_idx];
        obj.segments = segments;
        obj.sections = sections;

        obj_idx
    }

    /// Find the given symbol across the different loaded objects. Symbols with NDX set to
    /// [`SHN_UNDEF`](goblin::elf::section_header::SHN_UNDEF) are ignored.
    ///
    /// Applies the following priority:
    /// - Symbol with binding [`STB_LOCAL`] present in an [`SHT_STRTAB`](goblin::elf::section_header::SHT_STRTAB) section
    ///   of the object pointed at by `local`.
    /// - Symbol with binding [`STB_GLOBAL`] present in any [`SHT_DYNSYM`](goblin::elf::section_header::SHT_DYNSYM). Object
    ///   are searched in the same order as they were loaded.
    /// - Symbol with binding [`STB_WEAK`]. Same as above.
    pub fn find_symbol<'a>(
        &'a self,
        name: &'a CStr,
        local: Handle<Object>,
    ) -> Result<(&'a Section, Sym), FoldError> {
        // Search the local object for a `STB_LOCAL` entry.
        if let Ok((section, sym)) = self.objects[local].find_symbol(name, self) {
            if sym_bindings(&sym) == STB_LOCAL {
                return Ok((section, sym));
            }
        }

        let mut weak = Err(FoldError::SymbolNotFound(name.to_owned()));

        // Go through all loaded object to find a `STB_GLOBAL`, and stores the first `STB_WEAK`
        // in case no `STB_GLOBAL` is found.
        for (_, obj) in self.objects.enumerate() {
            if let Ok((section, sym)) = obj.find_dynamic_symbol(name, self) {
                match sym_bindings(&sym) {
                    STB_GLOBAL => return Ok((section, sym)),
                    STB_WEAK if weak.is_err() => weak = Ok((section, sym)),
                    _ => {}
                }
            }
        }

        weak
    }
}

// ———————————————————————————————— Indexing ———————————————————————————————— //

impl Index<Handle<Object>> for Manifold {
    type Output = Object;

    fn index(&self, handle: Handle<Object>) -> &Self::Output {
        &self.objects[handle]
    }
}

impl Index<Handle<Segment>> for Manifold {
    type Output = Segment;

    fn index(&self, handle: Handle<Segment>) -> &Self::Output {
        &self.segments[handle]
    }
}

impl Index<Handle<Section>> for Manifold {
    type Output = Section;

    fn index(&self, handle: Handle<Section>) -> &Self::Output {
        &self.sections[handle]
    }
}

impl IndexMut<Handle<Object>> for Manifold {
    fn index_mut(&mut self, handle: Handle<Object>) -> &mut Self::Output {
        &mut self.objects[handle]
    }
}

impl IndexMut<Handle<Segment>> for Manifold {
    fn index_mut(&mut self, handle: Handle<Segment>) -> &mut Self::Output {
        &mut self.segments[handle]
    }
}

impl IndexMut<Handle<Section>> for Manifold {
    fn index_mut(&mut self, handle: Handle<Section>) -> &mut Self::Output {
        &mut self.sections[handle]
    }
}
