use alloc::boxed::Box;
use alloc::sync::Arc;
use core::ffi::c_void;

use rustix::mm::{self, MapFlags, ProtFlags};

use crate::file::MappingMut;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::{Handle, Segment};

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

    fn process_segment(
        &mut self,
        segment: Handle<Segment>,
        fold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        let s = &fold.segments[segment];
        let o = &mut fold.objects[s.obj];
        log::info!("Loading segment of {}...", o.display_path());

        if s.mem_size == 0 {
            return Ok(());
        }

        let new_mapping = unsafe {
            let offset = o.pie_load_offset.unwrap_or(0);

            // Allocate memory
            let addr = s.vaddr + offset;

            let mapping = mm::mmap_anonymous(
                (addr & (!0xfff)) as *mut c_void,
                s.mem_size
                    + (addr & 0xfff)
                    + if addr == 0 {
                        let max = o
                            .segments
                            .iter()
                            .map(|s| &fold.segments[*s])
                            .max_by_key(|s| s.vaddr);
                        max.map(|s| s.vaddr + s.mem_size).unwrap_or(0)
                    } else {
                        0
                    },
                ProtFlags::READ | ProtFlags::WRITE | ProtFlags::EXEC,
                MapFlags::PRIVATE
                    | if addr == 0 {
                        MapFlags::empty()
                    } else {
                        MapFlags::FIXED
                    },
            )
            .expect("Anonymous mapping failed");

            log::info!("Segment loaded at 0x{:x}", mapping as usize);

            if s.vaddr == 0 && o.pie_load_offset.is_none() {
                o.pie_load_offset = Some(mapping as usize)
            }

            let mapping_start = mapping.add(addr & 0xfff);

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

            Arc::new(MappingMut::new(mapping_start as *mut u8, s.mem_size))
        };

        fold.segments[segment].loaded_mapping = Some(new_mapping);

        Ok(())
    }
}
