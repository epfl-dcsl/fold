#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

use fold::{exit, init_logging, Env, Exit};

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Trace);

    fold::default_chain("fold", env).run();

    exit(Exit::Success);
}
