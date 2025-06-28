#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

mod emulator;

use emulator::Emulator;
use fold::Filter;
use fold::Fold;

#[fold::chain]
fn emulator_chain(fold: Fold) -> Fold {
    fold.select("collect")
        .after()
        .register("architecture check", Emulator, Filter::any_object())
}
