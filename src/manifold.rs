use alloc::ffi::CString;
use alloc::sync::Arc;
use core::ops::Index;

use crate::arena::{Arena, Handle};
use crate::file::Mapping;
use crate::object::{Object, Section };

// ———————————————————————————————— Manifold ———————————————————————————————— //

/// The manifold is an intermediate representation of all objects composing a program.
pub struct Manifold {
    objects: Arena<Object>,
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

    pub(crate) fn add_elf_file(&mut self, file: Mapping, path: CString) {
        let file = Arc::new(file);
        let obj = Object::new(file.clone(), path);
        let obj_idx = self.objects.push(obj);
        let obj = &self[obj_idx];

        for segment in obj.program_headers() {
            log::info!("{segment:?}");
        }
        for section in obj.section_headers() {
            log::info!("{section:?}");
            Section::new(section, obj_idx, &self);
        }
    }
}

// ———————————————————————————————— Indexing ———————————————————————————————— //

impl Index<Handle<Object>> for Manifold {
    type Output = Object;

    fn index(&self, handle: Handle<Object>) -> &Self::Output {
        &self.objects[handle]
    }
}

