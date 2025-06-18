use alloc::boxed::Box;
use alloc::vec::Vec;

use fold::module::Module;
use syscalls::{syscall, Sysno};

#[derive(Debug)]
struct SysCollectError;

impl From<SysCollectError> for Box<dyn core::fmt::Debug> {
    fn from(value: SysCollectError) -> Self {
        Box::new(value)
    }
}

pub struct SysCollect;

impl Module for SysCollect {
    fn name(&self) -> &'static str {
        "syscall collect"
    }

    fn process_manifold(
        &mut self,
        manifold: &mut fold::manifold::Manifold,
    ) -> Result<(), alloc::boxed::Box<dyn core::fmt::Debug>> {
        // Combine filters for write and exit
        for obj in manifold.objects {
            obj.symbols().for_each(|s| println("{:}", s))
        }
    }
}
