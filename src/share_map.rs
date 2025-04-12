use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use core::any::Any;
use core::marker::PhantomData;

#[derive(Debug, Clone, Copy)]
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
pub struct ShareMap {
    map: BTreeMap<String, Box<dyn Any>>,
}

impl ShareMap {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert<T: 'static>(&mut self, key: ShareMapKey<T>, value: T) {
        self.map.insert(key.key.to_owned(), Box::new(value));
    }

    pub fn get<T: 'static>(&self, key: ShareMapKey<T>) -> Option<&T> {
        self.map.get(key.key).and_then(|v| v.downcast_ref())
    }
}
