#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

use fold::elf::cst::{PT_LOAD, SHT_RELA};
use fold::filters::{section, segment, ObjectFilter};
use fold::sysv::collector::SysvCollector;
use fold::sysv::loader::SysvLoader;
use fold::sysv::relocation::SysvReloc;
use fold::sysv::start::SysvStart;
use fold::{exit, init_logging, Env, Exit};

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Trace);

    fold::new(env)
        .search_path("/lib")
        .search_path("/lib64")
        .search_path("/usr/lib/")
        .search_path("/home/noe/EPFL/semester_project/dynamic-linker-project/samples")
        .phase("collect")
        .register(SysvCollector::new(), ObjectFilter::any())
        .phase("load dependencies")
        .register(SysvLoader::new(), ObjectFilter::any())
        .phase("load")
        .register(SysvLoader::new(), segment(PT_LOAD))
        .phase("relocation")
        .register(SysvReloc::new(), section(SHT_RELA))
        .phase("start")
        .register(SysvStart::new(), ObjectFilter::any())
        .run();

    exit(Exit::Success);
}
