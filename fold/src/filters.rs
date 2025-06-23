use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use core::ops::BitOr;

use crate::{Object, Section, Segment};

pub enum ItemFilter {
    Manifold,
    Object(Box<dyn Fn(&Object) -> bool>),
    Segment(Box<dyn Fn(&Object, &Segment) -> bool>),
    Section(Box<dyn Fn(&Object, &Section) -> bool>),
}

pub struct Filter {
    pub(crate) items: Vec<ItemFilter>,
}

impl Filter {
    pub fn matches_manifold(&self) -> bool {
        self.items.iter().any(|f| matches!(f, ItemFilter::Manifold))
    }

    pub fn matches_object(&self, object: &Object) -> bool {
        self.items.iter().any(|f| {
            if let ItemFilter::Object(f) = f {
                f(object)
            } else {
                false
            }
        })
    }

    pub fn is_segment_filter(&self) -> bool {
        self.items
            .iter()
            .any(|f| matches!(f, ItemFilter::Segment(_)))
    }
    pub fn matches_segment(&self, segment: &Segment, object: &Object) -> bool {
        self.items.iter().any(|f| {
            if let ItemFilter::Segment(f) = f {
                f(object, segment)
            } else {
                false
            }
        })
    }

    pub fn is_section_filter(&self) -> bool {
        self.items
            .iter()
            .any(|f| matches!(f, ItemFilter::Section(_)))
    }
    pub fn matches_section(&self, section: &Section, object: &Object) -> bool {
        self.items.iter().any(|f| {
            if let ItemFilter::Section(f) = f {
                f(object, section)
            } else {
                false
            }
        })
    }

    pub fn manifold() -> Self {
        Filter {
            items: vec![ItemFilter::Manifold],
        }
    }
    pub fn object<F: Fn(&Object) -> bool + 'static>(pred: F) -> Self {
        Filter {
            items: vec![ItemFilter::Object(Box::new(pred))],
        }
    }
    pub fn segment<F: Fn(&Object, &Segment) -> bool + 'static>(pred: F) -> Self {
        Filter {
            items: vec![ItemFilter::Segment(Box::new(pred))],
        }
    }
    pub fn section<F: Fn(&Object, &Section) -> bool + 'static>(pred: F) -> Self {
        Filter {
            items: vec![ItemFilter::Section(Box::new(pred))],
        }
    }

    pub fn any_object() -> Self {
        Self::object(|_| true)
    }
    pub fn any_segment() -> Self {
        Self::segment(|_, _| true)
    }
    pub fn any_section() -> Self {
        Self::section(|_, _| true)
    }

    pub fn segment_type(tag: u32) -> Filter {
        Self::segment(move |_, s| s.tag == tag)
    }
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
