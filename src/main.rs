#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

use fold::elf::cst::{PT_LOAD, SHT_FINI_ARRAY, SHT_INIT_ARRAY, SHT_RELA};
use fold::filters::{section, segment, ItemFilter, ObjectFilter};
use fold::sysv::collector::SysvRemappingCollector;
use fold::sysv::init_array::SysvInitArray;
use fold::sysv::loader::SysvLoader;
use fold::sysv::protect::SysvProtect;
use fold::sysv::relocation::{SysvReloc, SysvRelocLib};
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
        .register(
            SysvRemappingCollector::new()
                .replace_multiple(&[("libc.so", "libc.so"), ("libc.musl-x86_64.so", "libc.so")])
                .drop_multiple(&[
                    "ld-linux-x86-64.so",
                    "libcrypt.so",
                    "libdl.so",
                    "libm.so",
                    "libpthread.so",
                    "libresolv.so",
                    "librt.so",
                    "libutil.so",
                    "libxnet.so",
                ]),
            ObjectFilter::any(),
        )
        .phase("load")
        .register(SysvLoader, segment(PT_LOAD))
        .phase("tls")
        .register(SysvTls, ItemFilter::ManifoldFilter)
        .phase("relocation lib")
        .register(SysvRelocLib, section(SHT_RELA))
        .phase("relocation exe")
        .register(SysvReloc, section(SHT_RELA))
        .phase("protect")
        .register(SysvProtect, segment(PT_LOAD))
        .phase("init array")
        .register(SysvInitArray, section(SHT_INIT_ARRAY))
        .phase("fini array")
        .register(SysvInitArray, section(SHT_FINI_ARRAY))
        .phase("start")
        .register(SysvStart, ObjectFilter::any())
        .run();

    exit(Exit::Success);
}
