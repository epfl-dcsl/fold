#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

mod seccomp;

use fold::elf::cst::{PT_LOAD, SHT_FINI_ARRAY, SHT_INIT_ARRAY};
use fold::filters::{section, segment, ItemFilter, ObjectFilter};
use fold::sysv::collector::SysvRemappingCollector;
use fold::sysv::init_array::SysvInitArray;
use fold::sysv::loader::SysvLoader;
use fold::sysv::protect::SysvProtect;
use fold::sysv::relocation::SysvReloc;
use fold::sysv::start::SysvStart;
use fold::sysv::tls::SysvTls;
use fold::{exit, init_logging, Env, Exit};
use seccomp::Seccomp;

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
                .replace("libc.so", "libc.so")
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
        .phase("relocation")
        .register(
            SysvReloc::new(),
            ObjectFilter {
                mask: fold::filters::ObjectMask::Any, // TODO: match only elf
                os_abi: 0,
                elf_type: 0,
            },
        )
        .phase("protect")
        .register(SysvProtect, segment(PT_LOAD))
        .phase("init array")
        .register(SysvInitArray, section(SHT_INIT_ARRAY))
        .phase("fini array")
        .register(SysvInitArray, section(SHT_FINI_ARRAY))
        .phase("syscall restriction")
        .register(Seccomp, ItemFilter::ManifoldFilter)
        .phase("start")
        .register(SysvStart, ObjectFilter::any())
        .run();

    exit(Exit::Success);
}
