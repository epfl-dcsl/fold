use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use core::any::Any;
use core::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
/// A key into a [`ShareMap`].
///
/// The type `T` is the type of the corresponding value in the [`ShareMap`]. As a convention, [`ShareMapKey`]s should be
/// exposed as global constants to allow easy communication between modules.
pub struct ShareMapKey<T> {
    key: &'static str,
    _marker: PhantomData<T>,
}

impl<T> ShareMapKey<T> {
    pub const fn new(key: &'static str) -> Self {
        Self {
            key,
            _marker: PhantomData,
        }
    }
}

#[derive(Debug, Default)]
/// Shared memory to allow communication between [`Module`][crate::Module]s.
///
/// Entries are stored as a map between a [`ShareMapKey`] and any type. The [`ShareMapKey`]'s generic type must match
/// the type of the entry's value, ensuring typesafety.
pub struct ShareMap {
    map: BTreeMap<&'static str, Box<dyn Any>>,
}

impl ShareMap {
    pub(crate) fn new() -> Self {
        Default::default()
    }

    /// Insert a new value in the map, overwriting any values registered with the same key string.
    pub fn insert<T: 'static>(&mut self, key: ShareMapKey<T>, value: T) {
        self.map.insert(key.key, Box::new(value));
    }

    /// Retreives a value from the map. The type `T` of the `key` must match the type of the value in the map.
    pub fn get<T: 'static>(&self, key: ShareMapKey<T>) -> Option<&T> {
        self.map.get(key.key).and_then(|v| v.downcast_ref())
    }
}
