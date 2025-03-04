use alloc::{borrow::ToOwned, ffi::CString, format, vec::Vec};
use core::{ffi::CStr, fmt::Debug, mem};
use goblin::elf::{
    dynamic::{DT_NEEDED, DT_STRTAB},
    section_header::SHT_DYNAMIC,
};
use log::trace;

use crate::{file, manifold::Manifold, module::Module, Handle, Object};

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
    bytes: &'a [u8],
}

impl Iterator for ElfDynIter<'_> {
    type Item = (u64, u64);

    fn next(&mut self) -> Option<Self::Item> {
        if self.bytes.is_empty() {
            None
        } else {
            let u64size = mem::size_of::<u64>();

            let (int_bytes, rest) = self.bytes.split_at(u64size);
            self.bytes = rest;
            let tag = u64::from_le_bytes(int_bytes.try_into().unwrap());

            let (int_bytes, rest) = self.bytes.split_at(u64size);
            self.bytes = rest;
            let value = u64::from_le_bytes(int_bytes.try_into().unwrap());

            Some((tag, value))
        }
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

                    for (tag, value) in (ElfDynIter {
                        bytes: &sec.mapping.bytes()[sec.offset..sec.offset + sec.size],
                    }) {
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

            let file_fd = file::open_file_ro(
                format!(
                    "/home/ludovic/Desktop/epfl/cs498/dynamic-linker-project/samples/{}",
                    filename.to_str().unwrap()
                )
                .as_str(),
            )
            .expect("Target is not a file");
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
