#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

extern crate macros;

pub mod installer;

use core::ffi::CStr;

use fold::{
    exit,
    filters::Filter,
    init_logging, println, Env, Exit,
};
use macros::hook;

use crate::installer::TrampolineReloc;

#[hook]
fn puts_hook(str: *const i8) {
    println!(
        "[from hook]: puts called with \"{}\" !",
        unsafe { CStr::from_ptr(str) }.to_string_lossy()
    )
}

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Trace);

    fold::default_chain("trampoline", env)
        .apply("relocation", |p| {
            p.after().register(
                "hooks",
                TrampolineReloc::new().with_hook("puts", __puts_hook_trampoline),
                Filter::any_object(),
            )
        })
        .run();

    exit(Exit::Success);
}
