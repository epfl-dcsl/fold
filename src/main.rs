#![no_std]
#![no_main]

extern crate alloc;
extern crate fold;

use fold::elf::cst::PT_LOAD;
use fold::filters::{section, segment, ObjectFilter};
use fold::module::{CollectHandler, Module};
use fold::{exit, init_logging, Env, Exit, Handle, Object, Section, Segment};

fold::entry!(entry);

fn entry(env: Env) -> ! {
    init_logging(log::LevelFilter::Debug);

    fold::new(env)
        .search_path("/lib")
        .search_path("/lib64")
        .search_path("/usr/lib/")
        .phase("collect")
        .register(TestMod::new(), ObjectFilter::any())
        .register(TestMod::new(), segment(PT_LOAD))
        .register(TestMod::new(), section(0x42))
        .run();

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

    fn process_object(&mut self, obj: Handle<Object>, manifold: &mut fold::manifold::Manifold) {
        let obj = &manifold[obj];
        log::info!("Processing '{}'", obj.display_path());
    }

    fn process_segment(
        &mut self,
        segment: Handle<Segment>,
        manifold: &mut fold::manifold::Manifold,
    ) {
        let seg = &manifold[segment];
        log::info!("Processing segment with tag 0x{:x}", seg.tag);
    }

    fn process_section(
        &mut self,
        section: Handle<Section>,
        manifold: &mut fold::manifold::Manifold,
    ) {
        let sec = &manifold[section];
        log::info!("Processing section with tag 0x{:x}", sec.tag);
    }
}
