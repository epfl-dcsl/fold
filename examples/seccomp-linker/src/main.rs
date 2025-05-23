#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

mod seccomp;

use fold::filters::ItemFilter;
use fold::{exit, init_logging, Env, Exit};
use seccomp::Seccomp;

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Trace);

    fold::default_chain(env)
        .insert_phase_after("syscall restriction", "fini array")
        .register_in_phase("syscall restriction", Seccomp, ItemFilter::ManifoldFilter)
        .run();

    exit(Exit::Success);
}
