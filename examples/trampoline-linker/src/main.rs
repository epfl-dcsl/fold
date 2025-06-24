#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

extern crate tramp_macros;

pub mod installer;

use core::ffi::CStr;

use fold::{driver::Fold, filters::Filter, println};
use tramp_macros::hook;

use crate::installer::TrampolineReloc;

#[hook]
fn puts_hook(str: *const i8) {
    println!(
        "[from hook]: puts called with \"{}\" !",
        unsafe { CStr::from_ptr(str) }.to_string_lossy()
    )
}

#[fold::chain]
fn seccomp_chain(fold: Fold) -> Fold {
    fold.apply("relocation", |p| {
        p.after().register(
            "hooks",
            TrampolineReloc::new().with_hook("puts", __puts_hook_trampoline),
            Filter::any_object(),
        )
    })
}
