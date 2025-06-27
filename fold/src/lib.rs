#![no_std]
#![no_main]
#![allow(dead_code)]

extern crate alloc;
extern crate macros;

use core::panic::PanicInfo;

mod allocator;
mod cli;
mod driver;
mod env;
mod error;
mod exit;
mod filters;
mod manifold;
mod module;
mod object;
mod share_map;

pub mod arena;
pub mod elf;
pub mod file;
pub mod logging;
pub mod sysv;

pub use allocator::*;
pub use cli::*;
pub use driver::*;
pub use env::*;
pub use error::*;
pub use exit::*;
pub use filters::*;
pub use macros::chain;
pub use manifold::*;
pub use module::*;
pub use share_map::*;

pub use log;

#[macro_export]
/// Creates an entrypoint from a function receiving an [`Env`] as parameter. Superseeded by the [`chain`] macro.
///
/// ## Example
///
/// ```
/// fold::entry!(entry);
///
/// fn entry(env: fold::Env) -> ! {
///     fold::logging::init(log::LevelFilter::Trace);
///
///     fold::default_chain(env!("CARGO_BIN_NAME"), env).run();
///
///     fold::exit(fold::Exit::Success)
/// }
/// ```
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
