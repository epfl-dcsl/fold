use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::format;
use alloc::vec::Vec;
use core::ffi::CStr;
use core::fmt::Debug;

use goblin::elf::dynamic::DT_NEEDED;
use goblin::elf::section_header::{SHT_DYNAMIC, SHT_STRTAB};
use goblin::elf64::dynamic::Dyn;
use log::trace;
use rustix::fs;

use crate::elf::ElfItemIterator;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::{file, Handle, Object};

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

impl Module for SysvCollector {
    fn name(&self) -> &'static str {
        "sysv-collector"
    }

    fn process_object(
        &mut self,
        obj: Handle<Object>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        fn read_deps(obj: Handle<Object>, manifold: &mut Manifold) -> Vec<CString> {
            let mut deps = Vec::new();
            let obj = &manifold[obj];

            trace!("[{}] Collecting from obj", obj.display_path());

            for sec in obj.sections.iter() {
                let sec = manifold.sections.get(*sec).unwrap();

                if sec.tag == SHT_DYNAMIC {
                    let linked_dynstr = manifold
                        .get_section_link(sec)
                        .expect("Missing Dynstr entry");
                    assert!(linked_dynstr.tag == SHT_STRTAB);

                    for idx in ElfItemIterator::<Dyn>::from_section(sec)
                        .filter(|e| e.d_tag == DT_NEEDED)
                        .map(|e| e.d_val)
                    {
                        deps.push(CString::from(
                            CStr::from_bytes_until_nul(
                                &linked_dynstr.mapping.bytes()
                                    [(linked_dynstr.offset + idx as usize)..],
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

        Ok(())
    }
}
