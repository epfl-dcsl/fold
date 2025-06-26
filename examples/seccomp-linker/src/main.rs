#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

mod seccomp;

use fold::{Filter, Fold};
use seccomp::Seccomp;

#[fold::chain]
fn seccomp_chain(fold: Fold) -> Fold {
    fold.select("start")
        .before()
        .register("syscall restriction", Seccomp, Filter::manifold())
}
