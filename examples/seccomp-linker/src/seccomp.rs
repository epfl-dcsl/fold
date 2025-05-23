use alloc::boxed::Box;
use syscalls::{syscall, Sysno};

use fold::module::Module;

#[derive(Debug)]
struct SeccompError;

pub struct Seccomp;

impl From<SeccompError> for Box<dyn core::fmt::Debug> {
    fn from(value: SeccompError) -> Self {
        Box::new(value)
    }
}

impl Module for Seccomp {
    fn name(&self) -> &'static str {
        "seccomp"
    }

    fn process_manifold(
        &mut self,
        _manifold: &mut fold::manifold::Manifold,
    ) -> Result<(), alloc::boxed::Box<dyn core::fmt::Debug>> {
        unsafe {
            // SECCOMP_SET_MODE_STRICT mode, only allow read, write and exit
            syscall!(Sysno::seccomp, 0, 0, 0).map(|_| ()).map_err(|_| Box::from(SeccompError))
        }
    }
}
