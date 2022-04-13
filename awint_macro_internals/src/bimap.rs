use std::{
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
};

use triple_arena::{Arena, Ptr, PtrTrait};

/// This is a special purpose structure that can efficiently handle forwards and
/// backwards lookups, associates a unique id with each entry, associates a used
/// boolean with each entry, and maintains the set property (if multiple `T` are
/// inserted, only the first is retained).
///
/// Iteration over the arena is deterministic.
#[derive(Debug)]
pub struct BiMap<P: PtrTrait, T: Clone + Eq + Hash> {
    // TODO we need a more unified structure that can eliminate the extra `T`. There should be
    // another generic so that one of the generic types has the hashing and PartialEq applied to
    // it, while the other generic is for associated data

    // forwards lookup and set property
    map: HashMap<T, Ptr<P>>,
    // backwards lookup and determinism
    arena: Arena<P, (u64, T, bool)>,
    id: u64,
}

impl<P: PtrTrait, T: Clone + Eq + Hash> BiMap<P, T> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            arena: Arena::new(),
            id: 0,
        }
    }

    /// If `t` is already contained, it is not inserted
    pub fn insert(&mut self, t: T) -> Ptr<P> {
        match self.map.entry(t.clone()) {
            Entry::Occupied(o) => *o.get(),
            Entry::Vacant(v) => {
                let p = self.arena.insert((self.id, t, false));
                self.id += 1;
                v.insert(p);
                p
            }
        }
    }

    pub fn insert_get_ptr_and_id(&mut self, t: T) -> (Ptr<P>, u64) {
        match self.map.entry(t.clone()) {
            Entry::Occupied(o) => {
                let p = *o.get();
                (p, self.arena[p].0)
            }
            Entry::Vacant(v) => {
                let p = self.arena.insert((self.id, t, false));
                let res = self.id;
                self.id += 1;
                (*v.insert(p), res)
            }
        }
    }

    pub fn arena(&self) -> &Arena<P, (u64, T, bool)> {
        &self.arena
    }

    pub fn get_id<B: Borrow<T>>(&self, t: B) -> u64 {
        self.arena[self.map[t.borrow()]].0
    }

    pub fn get<B: Borrow<T>>(&self, t: B) -> (u64, Ptr<P>, bool) {
        let p = self.map[t.borrow()];
        let tmp = &self.arena[p];
        (tmp.0, p, tmp.2)
    }

    pub fn get_and_set_used<B: Borrow<T>>(&mut self, t: B) -> (u64, Ptr<P>) {
        let p = self.map[t.borrow()];
        self.arena[p].2 = true;
        let tmp = &self.arena[p];
        (tmp.0, p)
    }

    pub fn set_used<B: Borrow<Ptr<P>>>(&mut self, p: B) {
        self.arena[p].2 = true;
    }

    pub fn ptr_get_and_set_used<B: Borrow<Ptr<P>>>(&mut self, p: B) -> u64 {
        self.arena[*p.borrow()].2 = true;
        let tmp = &self.arena[*p.borrow()];
        tmp.0
    }

    /*pub fn get_p<B: Borrow<T>>(&self, t: B) -> Ptr<P> {
        self.map[t.borrow()]
    }

    pub fn get_t<B: Borrow<Ptr<P>>>(&self, p: B) -> (u64, &T) {
        let tmp = &self.arena[p];
        (tmp.0, &tmp.1)
    }*/
}
