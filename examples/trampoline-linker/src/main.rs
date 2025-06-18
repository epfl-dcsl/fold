#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

extern crate macros;

pub mod installer;

use core::ffi::CStr;

use fold::{
    exit,
    filters::{self, ObjectFilter},
    init_logging, println, Env, Exit,
};
use macros::hook;

use crate::installer::TrampolineReloc;

#[hook]
fn foo(str: *const i8) {
    println!(
        "[from hook]: puts called with \"{}\" !",
        unsafe { CStr::from_ptr(str) }.to_string_lossy()
    )
}

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Trace);

    fold::default_chain("trampoline", env)
        .insert_phase_after("hooks", "relocation")
        .register_in_phase(
            "hooks",
            TrampolineReloc::new().with_hook("puts", __foo_trampoline),
            ObjectFilter {
                mask: filters::ObjectMask::Any,
                os_abi: 0,
                elf_type: 0,
            },
        )
        .run();

    exit(Exit::Success);
}
