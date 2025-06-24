#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

mod seccomp;
mod syscall_collect;

use fold::driver::Fold;
use fold::filters::Filter;
use seccomp::Seccomp;
use syscall_collect::SysCollect;

#[fold::chain]
fn seccomp_chain(fold: Fold) -> Fold {
    fold.front()
        .register("syscall collect", SysCollect, Filter::any_object())
        .select("start")
        .before()
        .register("syscall restriction", Seccomp, Filter::manifold())
}
