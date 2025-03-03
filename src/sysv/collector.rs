use crate::{manifold::Manifold, module::Module, Handle, Object};

pub struct SysvCollector {}

impl SysvCollector {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SysvCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SysvCollector {
    fn name(&self) -> &'static str {
        "sysv-collector"
    }

    fn process_object(&mut self, obj: Handle<Object>, manifold: &mut Manifold) {
        let obj = &manifold[obj];
        log::info!("Processing '{}' (todo)", obj.display_path());
    }
}
