use alloc::boxed::Box;

use crate::elf::ElfItemIterator;
use crate::manifold::Manifold;
use crate::module::Module;
use crate::sysv::loader::SYSV_LOADER_BASE_ADDR;
use crate::Handle;

pub struct SysvInitArray {}

impl SysvInitArray {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SysvInitArray {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SysvInitArray {
    fn name(&self) -> &'static str {
        "sysv-init-array"
    }

    fn process_section(
        &mut self,
        section: Handle<crate::Section>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        let section = manifold.sections.get(section).unwrap();

        let base = *manifold
            .objects
            .get(section.obj)
            .unwrap()
            .shared
            .get(SYSV_LOADER_BASE_ADDR)
            .unwrap_or(&0) as u64;

        for addr in ElfItemIterator::<u64>::from_section(section) {
            let code: extern "C" fn() -> i64 = unsafe { core::mem::transmute(addr + base) };

            log::info!("Call function at {:x} (loaded at 0x{:x})", addr, addr + base);

            code();
        }

        Ok(())
    }
}
