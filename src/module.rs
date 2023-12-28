use crate::arena::Handle;
use crate::manifold::Manifold;
use crate::object::{Object, Section, Segment};

pub trait Module {
    fn name(&self) -> &'static str;

    fn process_object(&mut self, obj: Handle<Object>, manifold: &mut Manifold) {
        // Silence warnings
        let _ = obj;
        let _ = manifold;
        log::warn!(
            "Module '{}' does not implement 'process_object'",
            self.name()
        );
    }

    fn process_segment(&mut self, segment: Handle<Segment>, manifold: &mut Manifold) {
        // Silence warnings
        let _ = segment;
        let _ = manifold;
        log::warn!(
            "Module '{}' does not implement 'process_segment'",
            self.name()
        );
    }

    fn process_section(&mut self, section: Handle<Section>, manifold: &mut Manifold) {
        // Silence warnings
        let _ = section;
        let _ = manifold;
        log::warn!(
            "Module '{}' does not implement 'process_section'",
            self.name()
        );
    }
}
