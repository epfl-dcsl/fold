use crate::{file, manifold::Manifold, module::Module, Handle, Object};
use alloc::{ffi::CString, format, vec::Vec};
use core::{ffi::CStr, mem};
use goblin::elf::{
    dynamic::{DT_NEEDED, DT_STRTAB},
    section_header::SHT_DYNAMIC,
};
use log::{info, trace};

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

            trace!("[{}] Collecting", obj.display_path());

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

        let mut deps = Vec::new();
        let mut queue = Vec::new();

        queue.extend(read_deps(obj, manifold));

        while let Some(filename) = queue.pop() {
            if deps.contains(&filename) {
                continue;
            } else {
                deps.push(filename.clone());
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

            queue.extend(read_deps(obj, manifold));
        }

        info!("Found dependencies: {:#?}", read_deps(obj, manifold));
    }
}
