#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

use fold::elf::cst::PT_LOAD;
use fold::filters::{segment, ObjectFilter};
use fold::sysv::collector::SysvCollector;
use fold::sysv::loader::SysvLoader;
use fold::{exit, init_logging, Env, Exit};

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Debug);

    fold::new(env)
        .search_path("/lib")
        .search_path("/lib64")
        .search_path("/usr/lib/")
        .phase("collect")
        .register(SysvCollector::new(), ObjectFilter::any())
        .phase("load")
        .register(SysvLoader::new(), segment(PT_LOAD))
        .run();

    exit(Exit::Success);
}
