use alloc::{borrow::ToOwned, ffi::CString, format, vec::Vec};
use core::{ffi::CStr, fmt::Debug};
use goblin::elf::{
    dynamic::{DT_NEEDED, DT_STRTAB},
    section_header::SHT_DYNAMIC,
};
use log::trace;
use rustix::fs;

use crate::{bytes::BytesIter, file, manifold::Manifold, module::Module, Handle, Object};

#[derive(Clone)]
pub struct SysvCollectorEntry {
    pub name: CString,
    pub obj: Handle<Object>,
}

impl Debug for SysvCollectorEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SysvCollectorEntry")
            .field("name", &self.name.to_string_lossy())
            .field("obj", &"<handle>".to_owned())
            .finish()
    }
}

pub struct SysvCollectorResult {
    pub entries: Vec<SysvCollectorEntry>,
}

pub const SYSV_COLLECTOR_RESULT_KEY: &str = "sysv_collector";

pub struct SysvCollector {}

impl SysvCollector {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SysvCollector {
    fn default() -> Self {
        Self::new()
    }
}

struct ElfDynIter<'a> {
    bytes: BytesIter<'a>,
}

impl<'a> ElfDynIter<'a> {
    pub fn on_bytes(bytes: &'a [u8]) -> Self {
        Self {
            bytes: BytesIter { bytes },
        }
    }
}

impl Iterator for ElfDynIter<'_> {
    type Item = (u64, u64);

    fn next(&mut self) -> Option<Self::Item> {
        Some((self.bytes.read()?, self.bytes.read()?))
    }
}

impl Module for SysvCollector {
    fn name(&self) -> &'static str {
        "sysv-collector"
    }

    fn process_object(&mut self, obj: Handle<Object>, manifold: &mut Manifold) {
        fn read_deps(obj: Handle<Object>, manifold: &mut Manifold) -> Vec<CString> {
            let mut deps = Vec::new();
            let obj = &manifold[obj];

            trace!("[{}] Collecting from obj", obj.display_path());

            for sec in obj.sections.iter() {
                let sec: &crate::Section = &manifold.sections[*sec];

                if sec.tag == SHT_DYNAMIC {
                    let mut strtab = None;
                    let mut deps_idx = Vec::new();

                    for (tag, value) in ElfDynIter::on_bytes(
                        &sec.mapping.bytes()[sec.offset..sec.offset + sec.size],
                    ) {
                        match tag {
                            DT_STRTAB => strtab = Some(value),
                            DT_NEEDED => deps_idx.push(value),
                            _ => {}
                        }
                    }

                    let strtab = strtab.expect("Missing STRTAB entry");
                    for idx in deps_idx {
                        deps.push(CString::from(
                            CStr::from_bytes_until_nul(
                                &sec.mapping.bytes()[(strtab + idx) as usize..],
                            )
                            .expect("Invalid deps str"),
                        ));
                    }
                }
            }

            deps
        }

        let mut deps: Vec<SysvCollectorEntry> =
            match manifold.get_shared::<SysvCollectorResult>(SYSV_COLLECTOR_RESULT_KEY) {
                Some(scr) => scr.entries.clone(),
                None => Vec::new(),
            };

        let mut queue = Vec::new();

        queue.extend(read_deps(obj, manifold));

        while let Some(filename) = queue.pop() {
            if deps.iter().any(|e| e.name == filename) {
                continue;
            }

            let path_lib = manifold
                .search_paths
                .iter()
                .map(|p| format!("{}/{}", p, filename.to_str().unwrap()))
                .find(|p| fs::stat(p.as_str()).is_ok())
                .expect("Target not found");

            let file_fd = file::open_file_ro(path_lib.as_str()).expect("Target is not a file");

            let file = file::map_file(file_fd);
            let obj = manifold.add_elf_file(file, filename.clone());

            deps.push(SysvCollectorEntry {
                name: filename,
                obj,
            });

            queue.extend(read_deps(obj, manifold));
        }

        manifold.add_shared(
            SYSV_COLLECTOR_RESULT_KEY,
            SysvCollectorResult { entries: deps },
        );
    }
}
