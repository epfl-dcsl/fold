use alloc::boxed::Box;

use crate::arena::Handle;
use crate::manifold::Manifold;
use crate::object::{Object, Section, Segment};

/// A step of the linker's execution.
///
/// A module is applied to the [`Manifold`]'s elements, based on the [`Filter`][crate::Filter] it was registered with.
/// All methods have default implementation, such that a module may only be meaningfully applied to certain
/// [`Manifold`]'s elements.
pub trait Module {
    /// Returns a name to display for the module. Used for logging and debugging.
    fn name(&self) -> &'static str;

    /// Processes an object coming from the manifold.
    ///
    /// It is ensured that `obj` successfully indexes an element in `manifold.objects`. This function will never be
    /// called twice with the same objects.
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

    /// Processes an segment coming from the manifold.
    ///
    /// It is ensured that `segment` successfully indexes an element in `manifold.segments`. This function will never be
    /// called twice with the same segments.
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

    /// Processes an section coming from the manifold.
    ///
    /// It is ensured that `section` successfully indexes an element in `manifold.sections`. This function will never be
    /// called twice with the same sections.
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

    /// Processes the whole manifold.
    ///
    /// This function may be called at most once.
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
