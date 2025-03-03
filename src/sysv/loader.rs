use crate::{manifold::Manifold, module::Module, Handle, Segment};

pub struct SysvLoader {}

impl SysvLoader {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for SysvLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl Module for SysvLoader {
    fn name(&self) -> &'static str {
        "sysv-loader"
    }

    fn process_segment(&mut self, _segment: Handle<Segment>, _manifold: &mut Manifold) {
        log::info!("Loading segment...");
    }
}
