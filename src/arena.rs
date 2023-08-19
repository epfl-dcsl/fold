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

    pub fn enumerate(&self) -> HandleIter<'_, T> {
        HandleIter {
            inner: self.store.iter().enumerate(),
        }
    }

    pub fn enumerate_mut(&mut self) -> HandleIterMut<'_, T> {
        HandleIterMut {
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

pub struct HandleIter<'a, T> {
    inner: Enumerate<core::slice::Iter<'a, T>>,
}

impl<'a, T> Iterator for HandleIter<'a, T> {
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

pub struct HandleIterMut<'a, T> {
    inner: Enumerate<core::slice::IterMut<'a, T>>,
}

impl<'a, T> Iterator for HandleIterMut<'a, T> {
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
