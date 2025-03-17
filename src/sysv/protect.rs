use core::ffi::c_void;

use crate::{manifold::Manifold, module::Module, Handle, Segment};

use rustix::mm::{self, MprotectFlags, ProtFlags};

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

    fn process_segment(&mut self, segment: Handle<Segment>, fold: &mut Manifold) {
        log::info!("Loading segment...");

        let segment = fold.segments.get(segment).unwrap();

        if segment.mem_size == 0 {
            return;
        }

        unsafe {
            // Protect pages
            mm::mprotect(
                segment.mapping.bytes().as_ptr() as *mut c_void,
                segment.mem_size,
                MprotectFlags::from_bits(segment.flags).unwrap(),
            )
            .expect("Protecting pages failed");

            log::info!(
                "Segment from: 0x{:x} protected with prot: {:?}",
                segment.mapping.bytes().as_ptr() as usize,
                ProtFlags::from_bits(segment.flags).unwrap()
            );
        }
    }
}
