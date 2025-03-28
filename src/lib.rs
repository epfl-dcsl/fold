#![no_std]
#![no_main]
#![allow(dead_code)]

extern crate alloc;

use core::panic::PanicInfo;

mod allocator;
mod arena;
mod cli;
mod driver;
pub mod elf;
mod env;
pub mod error;
mod exit;
mod file;
pub mod filters;
mod logging;
pub mod manifold;
pub mod module;
mod object;
pub mod sysv;

pub use arena::Handle;
pub use driver::new;
pub use env::Env;
pub use exit::{exit, exit_error, Exit};
pub use logging::init as init_logging;
pub use object::{Object, section::Section, Segment};

#[macro_export]
macro_rules! entry {
    ($path:path) => {
        /// Dynamic linker entry point.
        #[no_mangle]
        unsafe extern "C" fn _start() {
            use core::arch::asm;

            asm! {
                "mov rdi, [rsp+8]",            // Get argc
                "mov rsi, rsp",
                "add rsi, 16",                 // Get pointer to argv
                "and rsp, 0xfffffffffffffff0", // Align stack to 16 bits
                "call {}",
                sym main,
                options(noreturn)
            }
        }

        extern "C" fn main(argc: usize, argv: usize) -> ! {
            // Typecheck user function
            let f: fn(env: $crate::Env) -> ! = $path;

            let env = unsafe { $crate::init(argv) };

            f(env);
        }
    };
}

/// Spidl initialization.
///
/// This function is not suppoded to be called by user code.
#[doc(hidden)]
pub unsafe fn init(argv: usize) -> Env {
    allocator::init_allocator();
    Env::from_argv(argv)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{}", info);
    exit(Exit::Error);
}
