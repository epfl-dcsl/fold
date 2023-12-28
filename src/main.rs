#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

use fold::elf::cst::PT_LOAD;
use fold::filters::{segment, ObjectFilter};
use fold::module::Module;
use fold::{exit, init_logging, Env, Exit, Handle, Object, Segment};

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

/// System V dependency collector.
///
/// Collects dynamic dependencies recursively.
struct SysvCollector {}

impl SysvCollector {
    fn new() -> Self {
        Self {}
    }
}

impl Module for SysvCollector {
    fn name(&self) -> &'static str {
        "sysv-collector"
    }

    fn process_object(&mut self, obj: Handle<Object>, manifold: &mut fold::manifold::Manifold) {
        let obj = &manifold[obj];
        log::info!("Processing '{}' (todo)", obj.display_path());
    }
}

/// System V loader.
///
/// Loads loadable segments in memory.
struct SysvLoader {}

impl SysvLoader {
    fn new() -> Self {
        Self {}
    }
}

impl Module for SysvLoader {
    fn name(&self) -> &'static str {
        "sysv-loader"
    }

    fn process_segment(
        &mut self,
        _segment: Handle<Segment>,
        _manifold: &mut fold::manifold::Manifold,
    ) {
        log::info!("Loading segment...");
    }
}
