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
        .search_path("/lib")
        .search_path("/lib64")
        .search_path("/usr/lib/")
        .register(TestMod::new(), segment(0x14))
        .register(TestMod::new(), section(0x42))
        .build()
        .load();

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

impl CollectHandler for TestMod {
    fn collect(&mut self, _manifold: &mut fold::manifold::Manifold) {
        todo!()
    }
}
