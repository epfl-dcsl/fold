#![no_std]
#![no_main]

extern crate alloc;

use core::panic::PanicInfo;

mod allocator;
mod cli;
mod driver;
mod env;
mod exit;
mod logging;
mod arena;
mod file;
pub mod manifold;
pub mod module;

pub use driver::new;
pub use env::Env;
pub use exit::{exit, exit_error, Exit};
pub use logging::init as init_logging;

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
