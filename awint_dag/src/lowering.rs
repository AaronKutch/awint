//! Special purpose arena

mod arena;
mod dag;
mod op;

use std::collections::BinaryHeap;

pub(crate) use arena::{Arena, Ptr};
pub(crate) use op::Op;

// TODO when `feature(binary_heap_into_iter_sorted)` is stabilized fix this hack
#[derive(Clone, Debug)]
pub(crate) struct IntoIterSorted<T> {
    inner: BinaryHeap<T>,
}

impl<T: Ord> Iterator for IntoIterSorted<T> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.inner.pop()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let exact = self.inner.len();
        (exact, Some(exact))
    }
}

pub(crate) fn into_iter_sorted<T>(heap: BinaryHeap<T>) -> IntoIterSorted<T> {
    IntoIterSorted { inner: heap }
}
