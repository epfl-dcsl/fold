#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

mod seccomp;
mod syscall_collect;

use fold::filters::Filter;
use fold::{Env, Exit, exit, init_logging};
use seccomp::Seccomp;
use syscall_collect::SysCollect;

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Trace);

    fold::default_chain("seccomp-sym-linker", env)
        .front()
        .register("syscall collect", SysCollect, Filter::any_object())
        .select("fini array")
        .before()
        .register("syscall restriction", Seccomp, Filter::manifold())
        .run();

    exit(Exit::Success);
}
