#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

use fold::manifold::{section, segment};
use fold::module::{CollectHandler, Module};
use fold::{exit, init_logging, Env, Exit};

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Debug);

    fold::new(env)
        .collect()
        .register(TestMod::new(), segment(0x14))
        .register(TestMod::new(), section(0x42));

    exit(Exit::Success);
}

struct TestMod {}

impl TestMod {
    fn new() -> Self {
        Self {}
    }
}

impl Module for TestMod {
    fn name(&self) -> &'static str {
        "testmod"
    }
}

impl CollectHandler for TestMod {}
