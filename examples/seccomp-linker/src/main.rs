#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

mod seccomp;
mod syscall_collect;

use fold::filters::{ItemFilter, ObjectFilter};
use fold::{exit, init_logging, Env, Exit};
use seccomp::Seccomp;
use syscall_collect::SysCollect;

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Trace);

    fold::default_chain("seccomp-linker", env)
        .push_front_phase("syscall collect")
        .register_in_phase("syscall collect", SysCollect, ObjectFilter::any())
        .insert_phase_after("syscall restriction", "fini array")
        .register_in_phase("syscall restriction", Seccomp, ItemFilter::ManifoldFilter)
        .run();

    exit(Exit::Success);
}
