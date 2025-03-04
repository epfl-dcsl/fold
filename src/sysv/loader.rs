use crate::{
    manifold::Manifold,
    module::Module,
    sysv::collector::{SysvCollectorResult, SYSV_COLLECTOR_RESULT_KEY},
    Handle, Segment,
};

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

    fn process_segment(&mut self, _segment: Handle<Segment>, fold: &mut Manifold) {
        log::info!("Loading segment...");
        let deps: &SysvCollectorResult = fold.get_shared(SYSV_COLLECTOR_RESULT_KEY).unwrap();

        for d in &deps.entries {
            log::info!("Loading deps {}", d.name.to_str().unwrap());
        }
    }
}
