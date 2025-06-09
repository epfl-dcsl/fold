use alloc::boxed::Box;

use crate::arena::Handle;
use crate::manifold::Manifold;
use crate::object::section::Section;
use crate::object::{Object, Segment};

pub trait Module {
    fn name(&self) -> &'static str;

    fn process_object(
        &mut self,
        obj: Handle<Object>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        // Silence warnings
        let _ = obj;
        let _ = manifold;
        log::warn!(
            "Module '{}' does not implement 'process_object'",
            self.name()
        );

        Ok(())
    }

    fn process_segment(
        &mut self,
        segment: Handle<Segment>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        // Silence warnings
        let _ = segment;
        let _ = manifold;
        log::warn!(
            "Module '{}' does not implement 'process_segment'",
            self.name()
        );

        Ok(())
    }

    fn process_section(
        &mut self,
        section: Handle<Section>,
        manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        // Silence warnings
        let _ = section;
        let _ = manifold;
        log::warn!(
            "Module '{}' does not implement 'process_section'",
            self.name()
        );

        Ok(())
    }

    fn process_manifold(
        &mut self,
        _manifold: &mut Manifold,
    ) -> Result<(), Box<dyn core::fmt::Debug>> {
        log::warn!(
            "Module '{}' does not implement 'process_manifold'",
            self.name()
        );

        Ok(())
    }
}
