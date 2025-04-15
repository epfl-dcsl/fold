use alloc::boxed::Box;
use core::ffi::c_void;

use rustix::mm::{self, MprotectFlags};

use crate::manifold::Manifold;
use crate::module::Module;
use crate::{Handle, Segment};

pub struct SysvProtect {}

impl SysvProtect {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SysvProtect {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SysvProtect {
    fn name(&self) -> &'static str {
        "sysv-protect"
    }

    fn process_segment(
        &mut self,
        segment: Handle<Segment>,
        fold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        log::info!("Protecting segment...");
        let segment = &fold.segments[segment];

        if let Some(mapping) = segment.loaded_mapping.as_ref() {
            if segment.mem_size == 0 {
                return Ok(());
            }

            unsafe {
                // Protect pages
                mm::mprotect(
                    (mapping.bytes().as_ptr() as usize & (!0xfff)) as *mut c_void,
                    segment.mem_size + (mapping.bytes().as_ptr() as usize & 0xfff),
                    MprotectFlags::from_bits(segment.flags).unwrap(),
                )
                .expect("Protecting pages failed");

                log::info!(
                    "Segment from: 0x{:x} protected with prot: {:?}",
                    mapping.bytes().as_ptr() as usize,
                    MprotectFlags::from_bits(segment.flags).unwrap()
                );
            }
        }

        Ok(())
    }
}
