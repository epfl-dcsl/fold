use alloc::boxed::Box;
use goblin::elf::program_header::{PF_R, PF_W, PF_X};
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

fn flags_to_prot(p_flags: u32) -> MprotectFlags {
    let mut prot = MprotectFlags::empty();
    if p_flags & PF_R != 0 {
        prot |= MprotectFlags::READ;
    }
    if p_flags & PF_W != 0 {
        prot |= MprotectFlags::WRITE;
    }
    if p_flags & PF_X != 0 {
        prot |= MprotectFlags::EXEC;
    }
    prot
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
                    flags_to_prot(segment.flags),
                )
                .expect("Protecting pages failed");

                log::info!(
                    "Segment from: 0x{:x} protected with prot: {:?} {}",
                    mapping.bytes().as_ptr() as usize,
                    flags_to_prot(segment.flags),
                    segment.flags
                );
            }
        }

        Ok(())
    }
}
