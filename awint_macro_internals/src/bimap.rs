use std::{
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
};

use triple_arena::{Arena, Ptr};

/// For results that have the same `Ok` and `Err` types
pub trait EitherResult {
    type T;
    fn either(self) -> Self::T;
}

impl<S> EitherResult for Result<S, S> {
    type T = S;

    fn either(self) -> Self::T {
        match self {
            Ok(t) => t,
            Err(t) => t,
        }
    }
}

/// This is a special purpose structure that can efficiently handle forwards and
/// backwards lookups and maintains the set property. `A` is associated data
/// that is not hashed or used in equality comparisons.
///
/// Iteration over the arena is deterministic.
#[derive(Debug)]
pub struct BiMap<P: Ptr, T: Clone + Eq + Hash, A> {
    // TODO we need a more unified structure that can eliminate the extra `T` with internal
    // memoization. In particular `insert_with` needs better optimization, maybe there needs to be
    // a "staged entry" for progressively inserting at different steps (start with &T to check for
    // uniqueness and avoid allocations, then move to T, then A). Probably need an evolution of
    // BTrees with higher radix trees and caches and quick defragmentation.

    // forwards lookup and set property
    map: HashMap<T, P>,
    // backwards lookup and determinism
    arena: Arena<P, (T, A)>,
}

impl<P: Ptr, T: Clone + Eq + Hash, A> BiMap<P, T, A> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            arena: Arena::new(),
        }
    }

    pub fn arena(&self) -> &Arena<P, (T, A)> {
        &self.arena
    }

    /// Warning: invalidating pointers in the arena can break the `BiMap`
    pub fn arena_mut(&mut self) -> &mut Arena<P, (T, A)> {
        &mut self.arena
    }

    pub fn contains(&self, t: &T) -> Option<P> {
        self.map.get(t).copied()
    }

    /*pub fn insert_with<P: Ptr, F0: FnOnce(P) -> T, F1: FnOnce() -> A>
      (&mut self, create: F0, associate: F1) -> P {
        self.arena.insert_with(|p| {
            let t = create(p);
            // need &T
            match self.map.entry(t.clone()) {
                Entry::Occupied(o) => *o.get(),
                Entry::Vacant(v) => {
                    let p = self.arena.insert((t, associate()));
                    v.insert(p);
                    p
                }
            }
        });
    }*/

    /// If `t` is already contained, it and `a` are not inserted. Returns `None`
    /// if inserted a new entry (use `F` to get the new `Ptr`), else returns
    /// the `Ptr` to an already existing `t`.
    pub fn insert_with<F: FnOnce(P) -> A>(&mut self, t: T, associate: F) -> Result<P, P> {
        match self.map.entry(t.clone()) {
            Entry::Occupied(o) => Err(*o.get()),
            Entry::Vacant(v) => {
                let p = self.arena.insert_with(|p| (t, associate(p)));
                v.insert(p);
                Ok(p)
            }
        }
    }

    pub fn insert(&mut self, t: T, a: A) -> Result<P, P> {
        self.insert_with(t, |_| a)
    }

    pub fn t_get<B: Borrow<T>>(&self, t: B) -> (P, &(T, A)) {
        let p = self.map[t.borrow()];
        (p, &self.arena[p])
    }

    pub fn p_get<B: Borrow<P>>(&self, p: B) -> &(T, A) {
        &self.arena[p]
    }

    pub fn a_get_mut<B: Borrow<P>>(&mut self, p: B) -> &mut A {
        &mut self.arena[p].1
    }
}

impl<P: Ptr, T: Clone + Eq + Hash, A> Default for BiMap<P, T, A> {
    fn default() -> Self {
        Self::new()
    }
}
