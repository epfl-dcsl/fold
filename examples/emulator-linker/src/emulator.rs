use alloc::boxed::Box;
use core::ffi::c_char;

use fold::Manifold;
use fold::Module;
use fold::arena::Handle;
use fold::elf::Object;
use syscalls::{Sysno, syscall};

pub const EM_RISCV: u16 = 0xf3;

#[derive(Debug)]
struct EmulationError;

impl From<EmulationError> for Box<dyn core::fmt::Debug> {
    fn from(value: EmulationError) -> Self {
        Box::new(value)
    }
}

pub struct Emulator;

impl Module for Emulator {
    fn name(&self) -> &'static str {
        "Architecture emulation"
    }

    fn process_object(
        &mut self,
        obj: Handle<Object>,
        manifold: &mut Manifold,
    ) -> Result<(), alloc::boxed::Box<dyn core::fmt::Debug>> {
        let obj = &manifold.objects[obj];

        if obj.e_machine == EM_RISCV {
            static PATH: &[u8] = b"/bin/qemu-riscv64-static\0";
            static ARG0: &[u8] = b"qemu-x86_64-static\0";
            static ARG1: &[u8] = b"-cpu\0";
            static ARG2: &[u8] = b"rv64\0";
            static ENV0: &[u8] = b"PATH=/usr/bin\0";

            let path = PATH.as_ptr() as *const c_char;
            let argv = [
                ARG0.as_ptr() as *const c_char,
                ARG1.as_ptr() as *const c_char,
                ARG2.as_ptr() as *const c_char,
                obj.display_path().as_ptr() as *const c_char,
                core::ptr::null(),
            ]
            .as_ptr();

            let envp = [ENV0.as_ptr() as *const c_char, core::ptr::null()].as_ptr();

            unsafe {
                syscall!(Sysno::execve, path as usize, argv as usize, envp as usize)
                    .map(|_| ())
                    .map_err(|_| Box::from(EmulationError))?;
            }
        }

        Ok(())
    }
}
