#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

mod seccomp;

use fold::filters::ItemFilter;
use fold::{Env, Exit, exit, init_logging};
use seccomp::Seccomp;

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Trace);

    fold::default_chain("seccomp-linker", env)
        .select("fini array")
        .after()
        .register("syscall restriction", Seccomp, ItemFilter::ManifoldFilter)
        .run();

    exit(Exit::Success);
}
