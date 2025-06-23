use alloc::boxed::Box;

use alloc::vec;
use alloc::vec::Vec;
use fold::Handle;
use fold::manifold::Manifold;
use fold::module::Module;
use fold::object::Object;
use fold::share_map::ShareMapKey;

const SYS_WRITE: u32 = 1;
const SYS_WRITEV: u32 = 20;
const SYS_IOCTL: u32 = 16;
const SYS_EXIT_GROUP: u32 = 231;

#[derive(Debug)]
struct SysCollectError;

impl From<SysCollectError> for Box<dyn core::fmt::Debug> {
    fn from(value: SysCollectError) -> Self {
        Box::new(value)
    }
}

pub const SECCOMP_SYSCALL_FILTER: ShareMapKey<Vec<u32>> =
    ShareMapKey::new("seccomp_syscall_filter");

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

        let mut filter = vec![];

        // Combine filters for write and exit
        for symbol in obj.symbols(manifold) {
            if let Ok((_sym, name)) = symbol
                && name.to_string_lossy().contains("puts")
            {
                filter.push(SYS_WRITEV);
                filter.push(SYS_WRITE);
                filter.push(SYS_IOCTL);
                filter.push(SYS_EXIT_GROUP);
            }
        }

        log::info!("Identified syscall(s) needed: {filter:?}");

        manifold.shared.insert(SECCOMP_SYSCALL_FILTER, filter);

        Ok(())
    }
}
