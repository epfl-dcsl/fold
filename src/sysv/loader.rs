use crate::{file::Mapping, manifold::Manifold, module::Module, Handle, Segment};
use core::ffi::c_void;

use alloc::sync::Arc;
use rustix::mm::{self, MapFlags, ProtFlags};

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

        let s = &mut fold.segments[segment];
        let o = fold.objects.get(s.obj).unwrap();

        if s.mem_size == 0 {
            return;
        }

        unsafe {
            let offset = fold.pie_load_offset.unwrap_or(0);

            // Allocate memory
            let addr = s.vaddr + offset;

            let mapping = mm::mmap_anonymous(
                (addr & (!0xfff)) as *mut c_void,
                s.mem_size + (addr & 0xfff),
                ProtFlags::WRITE,
                MapFlags::PRIVATE
                    | if addr == 0 {
                        MapFlags::empty()
                    } else {
                        MapFlags::FIXED
                    },
            )
            .expect("Anonymous mapping failed");

            log::info!("Segment loaded at 0x{:x}", mapping as usize);

            if s.vaddr == 0 && fold.pie_load_offset.is_none() {
                fold.pie_load_offset = Some(mapping as usize)
            }

            let mapping_start = mapping.add(addr & 0xfff);

            s.mapping = Arc::new(Mapping::new(mapping_start as *const u8, s.mem_size, None));

            // Copy segment data
            mapping_start.copy_from(
                ((o.raw().as_ptr() as usize) + s.offset) as *mut c_void,
                s.file_size,
            );

            if s.mem_size > s.file_size {
                // Zero memory after segment
                mapping_start
                    .add(s.file_size)
                    .write_bytes(0, s.mem_size - s.file_size);
            }
        }
    }
}
