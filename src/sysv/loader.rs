use crate::{manifold::Manifold, module::Module, println, Handle, Segment};
use core::ffi::c_void;

use log::info;
use rustix::mm::{self, MapFlags, MprotectFlags, ProtFlags};

use super::collector::{SysvCollectorResult, SYSV_COLLECTOR_RESULT_KEY};

pub struct SysvLoader {}

impl SysvLoader {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SysvLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SysvLoader {
    fn name(&self) -> &'static str {
        "sysv-loader"
    }

    fn process_object(&mut self, _obj: Handle<crate::Object>, fold: &mut Manifold) {
        log::info!("Loading dependencies...");
        let deps: &SysvCollectorResult = fold.get_shared(SYSV_COLLECTOR_RESULT_KEY).unwrap();

        for d in &deps.entries {
            log::info!("Loading deps {}", d.name.to_str().unwrap());
        }
    }

    fn process_segment(&mut self, segment: Handle<Segment>, fold: &mut Manifold) {
        log::info!("Loading segment...");

        let s = fold.segments.get(segment).unwrap();
        let o = fold.objects.get(s.obj).unwrap();

        assert!(s.mem_size > 0, "segment has size 0");

        unsafe {
            let offset = fold.pie_load_offset.unwrap_or(0);

            // Allocate memory
            let mapping = mm::mmap_anonymous(
                (s.vaddr + offset) as *mut c_void,
                s.mem_size,
                ProtFlags::WRITE, // ,
                MapFlags::PRIVATE,
            )
            .expect("Anonymous mapping failed");

            if s.vaddr == 0 && fold.pie_load_offset.is_none() {
                fold.pie_load_offset = Some(mapping as usize)
            }

            // Copy segment to mapped memory
            mapping.copy_from(
                ((o.raw().as_ptr() as usize) + s.offset) as *mut c_void,
                s.file_size,
            );

            if s.mem_size > s.file_size {
                // Zero memory after segment
                mapping
                    .add(s.file_size)
                    .write_bytes(0, s.mem_size - s.file_size);
            }

            // Protect pages
            mm::mprotect(
                mapping,
                s.mem_size,
                MprotectFlags::from_bits(s.flags).unwrap(),
            )
            .expect("Protecting pages failed");

            log::info!(
                "Segment from: 0x{:x} mapped to 0x{:x}  prot: {:?}",
                s.vaddr,
                mapping as usize,
                ProtFlags::from_bits(s.flags).unwrap()
            );
        }
    }
}
