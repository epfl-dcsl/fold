use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::ops::BitOr;

use crate::object::{Object, Section, Segment};

pub(crate) enum ItemFilter {
    Manifold,
    Object(Box<dyn Fn(&Object) -> bool>),
    Segment(Box<dyn Fn(&Object, &Segment) -> bool>),
    Section(Box<dyn Fn(&Object, &Section) -> bool>),
}

/// Element filter for applying [`Module`][crate::Module]s from a [`Fold`][crate::Fold].
///
/// A basic filter can match either the whole [`Manifold`][crate::Manifold], or [`Object`], [`Segment`] or [`Section`]
/// based on a predicate. `Filter` also exposes methods for usual cases, such as `any_*` or per-tag selection. `Filter`s
/// can be composed using the `|` ([`BitOr`]) operator to extend a [`Module`][crate::Module]'s reach.
/// 
/// ## Examples
/// 
/// ```
/// // Filter matching `PT_LOAD` segments
/// Filter::segment_type(PT_LOAD);
/// 
/// // Filter matching the manifold and objects 
/// Filter::manifold() | Filter::any_object();
/// ```
pub struct Filter {
    pub(crate) items: Vec<ItemFilter>,
}

impl Filter {
    pub(crate) fn matches_manifold(&self) -> bool {
        self.items.iter().any(|f| matches!(f, ItemFilter::Manifold))
    }

    pub(crate) fn matches_object(&self, object: &Object) -> bool {
        self.items.iter().any(|f| {
            if let ItemFilter::Object(f) = f {
                f(object)
            } else {
                false
            }
        })
    }

    pub(crate) fn is_segment_filter(&self) -> bool {
        self.items
            .iter()
            .any(|f| matches!(f, ItemFilter::Segment(_)))
    }
    pub(crate) fn matches_segment(&self, segment: &Segment, object: &Object) -> bool {
        self.items.iter().any(|f| {
            if let ItemFilter::Segment(f) = f {
                f(object, segment)
            } else {
                false
            }
        })
    }

    pub(crate) fn is_section_filter(&self) -> bool {
        self.items
            .iter()
            .any(|f| matches!(f, ItemFilter::Section(_)))
    }
    pub(crate) fn matches_section(&self, section: &Section, object: &Object) -> bool {
        self.items.iter().any(|f| {
            if let ItemFilter::Section(f) = f {
                f(object, section)
            } else {
                false
            }
        })
    }

    /// Creates a filter matching the [`Manifold`][crate::Manifold].
    pub fn manifold() -> Self {
        Filter {
            items: vec![ItemFilter::Manifold],
        }
    }
    /// Creates a filter matching [`Object`][crate::elf::Object]s based on a predicate.
    pub fn object<F: Fn(&Object) -> bool + 'static>(pred: F) -> Self {
        Filter {
            items: vec![ItemFilter::Object(Box::new(pred))],
        }
    }
    /// Creates a filter matching [`Segment`][crate::elf::Segment]s based on a predicate.
    pub fn segment<F: Fn(&Object, &Segment) -> bool + 'static>(pred: F) -> Self {
        Filter {
            items: vec![ItemFilter::Segment(Box::new(pred))],
        }
    }
    /// Creates a filter matching [`Section`][crate::elf::Section]s based on a predicate.
    pub fn section<F: Fn(&Object, &Section) -> bool + 'static>(pred: F) -> Self {
        Filter {
            items: vec![ItemFilter::Section(Box::new(pred))],
        }
    }

    /// Creates a filter matching all [`Object`][crate::elf::Object]s.
    pub fn any_object() -> Self {
        Self::object(|_| true)
    }
    /// Creates a filter matching all [`Segment`][crate::elf::Segment]s.
    pub fn any_segment() -> Self {
        Self::segment(|_, _| true)
    }
    /// Creates a filter matching all [`Section`][crate::elf::Section]s.
    pub fn any_section() -> Self {
        Self::section(|_, _| true)
    }

    /// Creates a filter matching [`Segment`][crate::elf::Segment]s of the given tag.
    pub fn segment_type(tag: u32) -> Filter {
        Self::segment(move |_, s| s.tag == tag)
    }
    /// Creates a filter matching [`Section`][crate::elf::Section]s of the given tag.
    pub fn section_type(tag: u32) -> Filter {
        Self::section(move |_, s| s.tag == tag)
    }
}

impl BitOr for Filter {
    type Output = Self;

    fn bitor(mut self, mut rhs: Self) -> Self::Output {
        self.items.append(&mut rhs.items);
        self
    }
}
