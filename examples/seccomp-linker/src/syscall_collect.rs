use alloc::boxed::Box;

use fold::manifold::Manifold;
use fold::module::Module;
use fold::object::Object;
use fold::Handle;
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

    fn process_object(
        &mut self,
        obj: Handle<Object>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        let obj = &manifold[obj];

        // Combine filters for write and exit
        obj.symbols(&manifold)
            .for_each(|s| log::info!("{:?}", s.unwrap().1));
        Ok(())
    }
}
