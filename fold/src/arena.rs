//! Wrapper around [`Vec`] that uses type-bound indexes.

use alloc::vec::Vec;
use core::cmp::{Eq, PartialEq};
use core::iter::{Enumerate, IntoIterator, Iterator};
use core::marker::PhantomData;
use core::ops::{Index, IndexMut};

// ————————————————————————————————— Arena —————————————————————————————————— //

/// Wrapper around [`Vec`] that uses type-bound indexes.
pub struct Arena<T> {
    store: Vec<T>,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    /// Creates an empty `Arena`.
    pub fn new() -> Self {
        Self { store: Vec::new() }
    }

    /// Adds `item` at the end of the Arena.
    pub fn push(&mut self, item: T) -> Handle<T> {
        let idx = self.store.len();
        self.store.push(item);
        Handle {
            idx,
            _marker: PhantomData,
        }
    }

    /// Returns the element at the given handle.
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        self.store.get(handle.idx)
    }

    /// Returns a mutable reference to the element at the given handle.
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        self.store.get_mut(handle.idx)
    }

    /// Return a handle iterator for the elements of the arena. The iterator will not stop at the end of the Arena and
    /// may therefore return invalid handles. Elements pushed into the arena during the iteration will be yielded.
    pub(crate) fn handle_generator(&self) -> HandleGenerator<T> {
        HandleGenerator::new()
    }

    /// Creates an [`EnumHandleIter`] over the arena.
    pub fn enumerate(&self) -> EnumHandleIter<'_, T> {
        EnumHandleIter {
            inner: self.store.iter().enumerate(),
        }
    }

    /// Creates an [`EnumHandleIterMut`] over the arena.
    pub fn enumerate_mut(&mut self) -> EnumHandleIterMut<'_, T> {
        EnumHandleIterMut {
            inner: self.store.iter_mut().enumerate(),
        }
    }
}

// ————————————————————————————————— Handle ————————————————————————————————— //

#[derive(Debug)]
/// Index into an [`Arena`]. The handle may not index an existing element of the [`Arena`], even it was created from the
/// same one.
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
        *self
    }
}

impl<T> Copy for Handle<T> {}

// —————————————————————————————————— Keys —————————————————————————————————— //

/// Trait used to index [`Arena`]s.
pub trait Key<T> {
    fn idx(self) -> usize;
}

impl<T> Key<T> for Handle<T> {
    fn idx(self) -> usize {
        self.idx
    }
}

// ——————————————————————————————— Iterators ———————————————————————————————— //

/// Handle generator for a given type `T`. It will infinitely yield handles with increasing indexes.
pub struct HandleGenerator<T> {
    idx: usize,
    _marker: PhantomData<T>,
}

impl<T> HandleGenerator<T> {
    fn new() -> Self {
        Self {
            idx: 0,
            _marker: PhantomData,
        }
    }
}

impl<T> Iterator for HandleGenerator<T> {
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

/// Iterator over an arena yielding both elements and their corresponding [`Handle`].
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

/// Iterator over an arena yielding both mutable references to elements and their corresponding [`Handle`].
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
