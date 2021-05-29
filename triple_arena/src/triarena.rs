use alloc::vec::Vec;
use core::{num::NonZeroU64, sync::atomic::Ordering};

use crate::{TriPtr, GLOBAL_ARENA_ID};

/// An arena containing elements (that can be non-`Clone`) that can be pointed
/// to by `TriPtr`s. This is a kind of generational arena that allows deletion
/// and inter-arena references (even between `Arena<T>`s of completely different
/// `T`).
#[derive(Debug)] // TODO make `Debug` of `m` nicer, keep other information
pub struct Arena<T> {
    /// Unique arena id to allow for safe inter-arena references
    id: NonZeroU64,
    /// Generation, starts at 1 and increments for every invalidation of a
    /// `TriPtr` to this arena
    gen: NonZeroU64,
    /// Number of elements currently contained in the arena
    len: usize,
    /// The main memory of entries. `m.len()` is always 0 or a power of 2.
    m: Vec<Entry<T>>,
    /// Range of `free_tracker`s that consist of unallocated elements.
    /// `free_range.0` denotes the inclusive start of this range and is
    /// always `0 <= free_range.0 < m.len()`. `free_range.1` is the noninclusive
    /// end of the range. The end of the range may be before the start,
    /// which indicates the range wraps around. If `self.len != self.m.len()`
    /// and `free_range.0 == free_range.1`, then it actually indicates that all
    /// of the entries are free instead of none of the entries being free.
    free_range: (usize, usize),
}

/// Internal entry for a `Arena`
#[derive(Debug)]
struct Entry<T> {
    /// This is used by the arena for tracking what entries are internally
    /// allocated or not. This can be completely unrelated to what is stored in
    /// this specific entry.
    free_tracker: usize,
    /// Generation and data. If `None`, then this is not internally allocated.
    /// Note: because the tuple has a `NonZeroU64`, the enum tag optimization is
    /// applied.
    data: Option<(NonZeroU64, T)>,
}

// This is a free function because of borrowing issues with `self`
/// Take a generation, increment it (and panic in case of overflow), and return
/// the new generation
#[inline]
fn inc_gen(x: NonZeroU64) -> NonZeroU64 {
    match NonZeroU64::new(x.get().wrapping_add(1)) {
        Some(x) => x,
        None => panic!("generation overflow"),
    }
}

/// # Note
///
/// A `TriPtr` is invalid if:
///  - it points to a different arena than the one it is being used as an
///    argument to
///  - it points to an element that has been `remove`d or has otherwise been the
///    target of some pointer invalidation operation
impl<T> Arena<T> {
    /// Creates a new arena that can contain elements of type `T`. Note: this
    /// function makes atomic fetches.
    pub fn new() -> Arena<T> {
        // We want the most relaxed ordering while still practically checking for
        // overflow.
        let new_id = GLOBAL_ARENA_ID.fetch_add(1, Ordering::Relaxed);
        if new_id <= 0 {
            panic!("GLOBAL_ARENA_ID overflow");
        }
        Arena {
            id: NonZeroU64::new(new_id as u64).unwrap(),
            gen: NonZeroU64::new(1).unwrap(),
            len: 0,
            m: Vec::new(),
            free_range: (0, 0),
        }
    }

    /// Returns the unique id of this arena
    pub fn id(&self) -> NonZeroU64 {
        self.id
    }

    /// Returns the number of elements in the arena
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns if the arena is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the capacity of the arena
    pub fn capacity(&self) -> usize {
        self.m.len()
    }

    /// Tries to insert element `e` into the arena without changing its
    /// capacity.
    ///
    /// # Errors
    ///
    /// Returns ownership of `e` if there are no remaining unallocated entries
    /// in the arena.
    pub fn try_insert(&mut self, e: T) -> Result<TriPtr, T> {
        if self.len == self.m.len() {
            Err(e)
        } else {
            // find next free entry from the free queue
            let index = self.m[self.free_range.0].free_tracker;
            self.m[index].data = Some((self.gen, e));
            // `m.len()` is a power of two, so this branchless wraparound works
            self.free_range.0 = self.free_range.0.wrapping_add(1) & (self.m.len().wrapping_sub(1));
            self.len += 1;
            Ok(TriPtr {
                arena_id: self.id,
                gen: self.gen,
                index,
            })
        }
    }

    /// Tries to insert the element created by `create` into the arena without
    /// changing its capacity.
    ///
    /// # Errors
    ///
    /// Does not run `create` and returns ownership if there are no remaining
    /// unallocated entries in the arena.
    pub fn try_insert_with<F: FnOnce() -> T>(&mut self, create: F) -> Result<TriPtr, F> {
        if self.len == self.m.len() {
            Err(create)
        } else {
            let index = self.m[self.free_range.0].free_tracker;
            self.m[index].data = Some((self.gen, create()));
            self.free_range.0 = self.free_range.0.wrapping_add(1) & (self.m.len().wrapping_sub(1));
            self.len += 1;
            Ok(TriPtr {
                arena_id: self.id,
                gen: self.gen,
                index,
            })
        }
    }

    /// Inserts element `e` into the arena and returns a `TriPtr` to it. If more
    /// capacity is needed, the Arena reallocates in powers of two.
    pub fn insert(&mut self, e: T) -> TriPtr {
        match self.try_insert(e) {
            Ok(index) => index,
            Err(e) => {
                let m_len = self.m.len();
                self.m.reserve_exact(m_len);
                self.m.push(Entry {
                    free_tracker: 0, // unused
                    data: Some((self.gen, e)),
                });
                for i in 1..m_len {
                    // `self.free_range` must be an empty range when this branch is reached,
                    // we can track all the new internally unallocated entries with the
                    // newly pushed elements.
                    self.m.push(Entry {
                        free_tracker: m_len + i,
                        data: None,
                    });
                }
                self.free_range = if m_len < 2 {
                    // all entries are allocated
                    (0, 0)
                } else {
                    // set the range to be all the entries we just reserved after the newly
                    // allocated one
                    (m_len + 1, 0)
                };
                self.len += 1;
                TriPtr {
                    arena_id: self.id,
                    gen: self.gen,
                    index: m_len,
                }
            }
        }
    }

    /// Returns an immutable reference to the element pointed to by `p`. Returns
    /// `None` if `p` is invalid.
    pub fn get(&self, p: TriPtr) -> Option<&T> {
        let tmp = self.m.get(p.index)?.data.as_ref()?;
        if self.id == p.arena_id && tmp.0 == p.gen {
            Some(&tmp.1)
        } else {
            None
        }
    }

    /// Returns a mutable reference to the element pointed to by `p`. Returns
    /// `None` if `p` is invalid.
    pub fn get_mut(&mut self, p: TriPtr) -> Option<&mut T> {
        let tmp = self.m.get_mut(p.index)?.data.as_mut()?;
        if self.id == p.arena_id && tmp.0 == p.gen {
            Some(&mut tmp.1)
        } else {
            None
        }
    }

    /// Invalidates all references to the element pointed to by `p`, and returns
    /// a new valid reference. Does no invalidation and returns `None` if `p` is
    /// invalid.
    pub fn invalidate(&mut self, p: TriPtr) -> Option<TriPtr> {
        let mut p = p;
        let tmp = self.m.get_mut(p.index)?.data.as_mut()?;
        if self.id == p.arena_id && tmp.0 == p.gen {
            let new_gen = inc_gen(self.gen);
            tmp.0 = new_gen;
            p.gen = new_gen;
            self.gen = new_gen;
            Some(p)
        } else {
            None
        }
    }

    /// Replaces the element pointed to by `p` with `new`, returns the old
    /// element, and keeps the internal generation counter as-is so that
    /// previously constructed `TriPtr`s to this entry are still valid.
    ///
    /// # Errors
    ///
    /// Returns ownership of `new` instead if `p` is invalid
    pub fn replace_keep_gen(&mut self, p: TriPtr, new: T) -> Result<T, T> {
        if let Some(tmp) = self.m.get_mut(p.index) {
            if let Some((gen, old)) = tmp.data.take() {
                if self.id == p.arena_id && gen == p.gen {
                    tmp.data = Some((gen, new));
                    return Ok(old)
                } else {
                    // do not drop `old` if invalid
                    tmp.data = Some((gen, old));
                }
            }
        }
        Err(new)
    }

    /// Replaces the element pointed to by `p` with `new`, returns a tuple of
    /// the new pointer and old element, and updates the internal generation
    /// counter so that previously constructed `TriPtr`s to this entry are
    /// invalidated.
    ///
    /// # Errors
    ///
    /// Does no invalidation and returns ownership of `new` instead if `p` is
    /// invalid
    pub fn replace_update_gen(&mut self, p: TriPtr, new: T) -> Result<(TriPtr, T), T> {
        let mut p = p;
        if let Some(tmp) = self.m.get_mut(p.index) {
            if let Some((old_gen, old)) = tmp.data.take() {
                if self.id == p.arena_id && old_gen == p.gen {
                    let new_gen = inc_gen(self.gen);
                    tmp.data = Some((new_gen, new));
                    self.gen = new_gen;
                    p.gen = new_gen;
                    return Ok((p, old))
                } else {
                    // do not drop `old` if invalid
                    tmp.data = Some((old_gen, old));
                }
            }
        }
        Err(new)
    }

    /// Removes the element pointed to by `p`, returns the element, and
    /// invalidates old `TriPtr`s to the element. Does no invalidation and
    /// returns `None` if `p` is invalid.
    pub fn remove(&mut self, p: TriPtr) -> Option<T> {
        let tmp = self.m.get_mut(p.index)?;
        let (gen, e) = tmp.data.take()?;
        if self.id == p.arena_id && gen == p.gen {
            self.gen = inc_gen(self.gen);
            // add to the end of the free queue
            self.m[self.free_range.1].free_tracker = p.index;
            // `m.len()` is a power of two, so this branchless wraparound works
            self.free_range.1 = self.free_range.1.wrapping_add(1) & (self.m.len().wrapping_sub(1));
            self.len -= 1;
            Some(e)
        } else {
            // do not drop `e` if invalid
            tmp.data = Some((gen, e));
            None
        }
    }

    /// Clears all elements from the arena and invalidates all pointers
    /// previously created from it. Note that this has no effect on allocated
    /// capacity.
    pub fn clear(&mut self) {
        // drop all `T`
        self.m.iter_mut().for_each(|x| {
            let _ = x.data.take();
        });
        self.gen = inc_gen(self.gen);
        self.len = 0;
        self.free_range = (0, 0);
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}
