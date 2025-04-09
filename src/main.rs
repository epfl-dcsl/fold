#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

use fold::elf::cst::{PT_LOAD, SHT_RELA};
use fold::filters::{section, segment, ItemFilter, ObjectFilter};
use fold::sysv::collector::SysvCollector;
use fold::sysv::loader::SysvLoader;
use fold::sysv::protect::SysvProtect;
use fold::sysv::relocation::SysvReloc;
use fold::sysv::start::SysvStart;
use fold::sysv::tls::SysvTls;
use fold::{exit, init_logging, Env, Exit};

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Trace);

    fold::new(env)
        .search_path("./musl/lib/")
        .search_path("/lib")
        .search_path("/lib64")
        .search_path("/usr/lib/")
        .phase("collect")
        .register(SysvCollector::new(), ObjectFilter::any())
        .phase("load")
        .register(SysvLoader::new(), segment(PT_LOAD))
        .phase("relocation")
        .register(SysvReloc::new(), section(SHT_RELA))
        .phase("protect")
        .register(SysvProtect::new(), segment(PT_LOAD))
        .phase("tls")
        .register(SysvTls::new(), ItemFilter::ManifoldFilter)
        .phase("start")
        .register(SysvStart::new(), ObjectFilter::any())
        .run();

    exit(Exit::Success);
}
