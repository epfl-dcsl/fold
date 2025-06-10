#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

mod emulator;

use fold::filters::ObjectFilter;
use fold::{exit, init_logging, Env, Exit};
use emulator::Emulator;

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Trace);

    fold::default_chain("emulator-linker", env)
        .insert_phase_before("architecture check", "collect")
        .register_in_phase("architecture check", Emulator, ObjectFilter::any())
        .run();

    exit(Exit::Success);
}