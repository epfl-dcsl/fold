#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

mod emulator;

use emulator::Emulator;
use fold::filters::Filter;
use fold::{Env, Exit, exit, init_logging};

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Trace);

    fold::default_chain("emulator-linker", env)
        .select("collect")
        .after()
        .register("architecture check", Emulator, Filter::any_object())
        .run();

    exit(Exit::Success);
}
