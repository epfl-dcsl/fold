use alloc::vec::Vec;
use core::cmp::{Eq, PartialEq};
use core::iter::{Enumerate, IntoIterator, Iterator};
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

// ————————————————————————————————— Arena —————————————————————————————————— //

pub struct Arena<T> {
    store: Vec<T>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self { store: Vec::new() }
    }

    pub fn push(&mut self, item: T) -> Handle<T> {
        let idx = self.store.len();
        self.store.push(item);
        Handle {
            idx,
            _marker: PhantomData,
        }
    }

    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.store.get(handle.idx)
    }

    /// Retur an handle iterator.
    /// The iterator does not borrow self, but does not guarantee handle validity. Therefore,
    /// handles returned by this handler can be invalid.
    pub(crate) fn handle_generator(&self) -> HandleIter<T> {
        HandleIter::new()
    }

    pub fn enumerate(&self) -> EnumHandleIter<'_, T> {
        EnumHandleIter {
            inner: self.store.iter().enumerate(),
        }
    }

    pub fn enumerate_mut(&mut self) -> EnumHandleIterMut<'_, T> {
        EnumHandleIterMut {
            inner: self.store.iter_mut().enumerate(),
        }
    }
}

// ————————————————————————————————— Handle ————————————————————————————————— //

#[derive(Debug)]
pub struct Handle<T> {
    idx: usize,
    _marker: PhantomData<T>,
}

impl<T> Handle<T> {
    /// An invalid handle that will cause a panic if used to access an object.
    pub const INVALID: Self = Self {
        idx: usize::MAX,
        _marker: PhantomData,
    };

    pub fn idx(self) -> usize {
        self.idx
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}

impl<T> Eq for Handle<T> {}

impl<T, K> Index<K> for Arena<T>
where
    K: Key<T>,
{
    type Output = T;

    fn index(&self, key: K) -> &T {
        &self.store[key.idx()]
    }
}

impl<T, K> IndexMut<K> for Arena<T>
where
    K: Key<T>,
{
    fn index_mut(&mut self, key: K) -> &mut Self::Output {
        &mut self.store[key.idx()]
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            idx: self.idx,
            _marker: PhantomData,
        }
    }
}

impl<T> Copy for Handle<T> {}

// —————————————————————————————————— Keys —————————————————————————————————— //

pub trait Key<T> {
    fn idx(self) -> usize;
}

impl<T> Key<T> for Handle<T> {
    fn idx(self) -> usize {
        self.idx
    }
}

// ——————————————————————————————— Iterators ———————————————————————————————— //

pub struct HandleIter<T> {
    idx: usize,
    _marker: PhantomData<T>,
}

impl<T> HandleIter<T> {
    fn new() -> Self {
        Self {
            idx: 0,
            _marker: PhantomData,
        }
    }
}

impl<T> Iterator for HandleIter<T> {
    type Item = Handle<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;
        Some(Handle {
            idx,
            _marker: PhantomData,
        })
    }
}

pub struct EnumHandleIter<'a, T> {
    inner: Enumerate<core::slice::Iter<'a, T>>,
}

impl<'a, T> Iterator for EnumHandleIter<'a, T> {
    type Item = (Handle<T>, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(idx, item)| {
            (
                Handle {
                    idx,
                    _marker: PhantomData,
                },
                item,
            )
        })
    }
}

pub struct EnumHandleIterMut<'a, T> {
    inner: Enumerate<core::slice::IterMut<'a, T>>,
}

impl<'a, T> Iterator for EnumHandleIterMut<'a, T> {
    type Item = (Handle<T>, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(idx, item)| {
            (
                Handle {
                    idx,
                    _marker: PhantomData,
                },
                item,
            )
        })
    }
}

impl<'a, T> IntoIterator for &'a Arena<T> {
    type Item = &'a T;
    type IntoIter = core::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.store.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut Arena<T> {
    type Item = &'a mut T;
    type IntoIter = core::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.store.iter_mut()
    }
}
