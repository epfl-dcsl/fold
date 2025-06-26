#![no_std]
#![no_main]
#![allow(dead_code)]

extern crate alloc;
extern crate macros;

use core::panic::PanicInfo;

mod cli;
mod driver;
mod env;
mod exit;
mod object;
mod share_map;

mod allocator;
pub mod arena;
pub mod elf;
mod error;
pub mod file;
pub mod filters;
pub mod logging;
mod manifold;
mod module;
pub mod sysv;

pub use allocator::*;
pub use cli::*;
pub use driver::*;
pub use env::*;
pub use error::*;
pub use exit::*;
pub use macros::chain;
pub use manifold::*;
pub use module::*;
pub use share_map::*;

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
    println!("{}", "hi");
    Env::from_argv(argv)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    log::error!("{info}");
    exit(Exit::Error);
}
