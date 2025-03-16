use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::ffi::CString;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::any::{Any, TypeId};
use core::ops::Index;

use crate::arena::{Arena, Handle};
use crate::file::Mapping;
use crate::object::{Object, Section, Segment};

// ———————————————————————————————— Manifold ———————————————————————————————— //

/// The manifold is an intermediate representation of all objects composing a program.
pub struct Manifold {
    pub objects: Arena<Object>,
    pub sections: Arena<Section>,
    pub segments: Arena<Segment>,
    pub shared: BTreeMap<TypeId, BTreeMap<String, Box<dyn Any>>>,
    pub regions: Arena<()>,
    pub search_paths: Vec<String>,
    pub pie_load_offset: Option<usize>,
}

impl Manifold {
    pub(crate) fn new() -> Self {
        Self {
            objects: Arena::new(),
            sections: Arena::new(),
            segments: Arena::new(),
            regions: Arena::new(),
            shared: BTreeMap::new(),
            search_paths: Vec::new(),
            pie_load_offset: None,
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

    pub fn add_shared<T: 'static>(&mut self, key: &str, value: T) {
        let type_id = TypeId::of::<T>();

        if let Some(entry) = self.shared.get_mut(&type_id) {
            entry.insert(key.to_owned(), Box::new(value));
        } else {
            let mut map: BTreeMap<String, Box<dyn Any>> = BTreeMap::new();
            map.insert(key.to_owned(), Box::new(value));
            self.shared.insert(type_id, map);
        }
    }

    pub fn get_shared<T: 'static>(&self, key: &str) -> Option<&T> {
        self.shared
            .get(&TypeId::of::<T>())
            .and_then(|v| v.get(key))
            .and_then(|v| v.downcast_ref())
    }

    pub fn add_search_paths(&mut self, paths: Vec<String>) {
        self.search_paths.extend(paths);
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
