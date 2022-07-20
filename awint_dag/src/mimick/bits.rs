use std::{fmt, mem, num::NonZeroUsize, ptr, rc::Rc};

use crate::{
    common::{Lineage, Op, State},
    mimick::{ExtAwi, InlAwi},
};

// this is a workaround for https://github.com/rust-lang/rust/issues/57749 that works on stable
// TODO fix when PR 83850 is merged

#[derive(Debug)]
pub(in crate::mimick) struct InnerState(pub(crate) Rc<State>);

/// Mimicking `awint_core::Bits`
#[repr(transparent)] // for the transmute
pub struct Bits {
    // use different names for the different raw `InnerState`s, or else Rust can think we are
    // trying to go through the `Deref` impls
    _bits_raw: [InnerState],
}

// Safety: `Bits` follows standard slice initialization invariants and is marked
// `#[repr(transparent)]`. The explicit lifetimes make sure they do not become
// unbounded.

impl<'a> Bits {
    /// Assumes this is called on a pointer from a `[InnerState; 1]`
    unsafe fn from_raw_parts(raw_ptr: *const InnerState) -> &'a Self {
        unsafe { mem::transmute::<&[InnerState], &Bits>(&*ptr::slice_from_raw_parts(raw_ptr, 1)) }
    }

    /// Assumes this is called on a pointer from a `[InnerState; 1]`
    unsafe fn from_raw_parts_mut(raw_ptr: *mut InnerState) -> &'a mut Self {
        unsafe {
            mem::transmute::<&mut [InnerState], &mut Bits>(&mut *ptr::slice_from_raw_parts_mut(
                raw_ptr, 1,
            ))
        }
    }
}

impl<'a> ExtAwi {
    pub fn const_as_ref(&'a self) -> &'a Bits {
        unsafe { Bits::from_raw_parts(self._extawi_raw.as_ptr()) }
    }

    pub fn const_as_mut(&'a mut self) -> &'a mut Bits {
        unsafe { Bits::from_raw_parts_mut(self._extawi_raw.as_mut_ptr()) }
    }
}

impl<'a, const BW: usize, const LEN: usize> InlAwi<BW, LEN> {
    pub fn const_as_ref(&'a self) -> &'a Bits {
        unsafe { Bits::from_raw_parts(self._inlawi_raw.as_ptr()) }
    }

    pub fn const_as_mut(&'a mut self) -> &'a mut Bits {
        unsafe { Bits::from_raw_parts_mut(self._inlawi_raw.as_mut_ptr()) }
    }
}

impl Lineage for &Bits {
    fn hidden_const_nzbw() -> Option<NonZeroUsize> {
        None
    }

    fn state(&self) -> Rc<State> {
        Rc::clone(&self._bits_raw[0].0)
    }
}

impl Lineage for &mut Bits {
    fn hidden_const_nzbw() -> Option<NonZeroUsize> {
        None
    }

    fn state(&self) -> Rc<State> {
        Rc::clone(&self._bits_raw[0].0)
    }
}

impl Bits {
    // TODO if we use dynamic bitwidths do we do something like this?
    /*
    pub fn bw(&self) -> prim::usize {
        prim::usize::new(BwAssign, vec![self.state()])
    }
    */

    pub fn nzbw(&self) -> NonZeroUsize {
        self.state().nzbw.unwrap()
    }

    pub fn bw(&self) -> usize {
        self.nzbw().get()
    }

    pub fn const_as_ref(&self) -> &Self {
        self
    }

    pub fn const_as_mut(&mut self) -> &mut Self {
        self
    }

    pub fn update_state(&mut self, nzbw: Option<NonZeroUsize>, op: Op<Rc<State>>) {
        // other `Rc`s that need the old state will keep it alive despite this one being
        // dropped
        let _: Rc<State> = mem::replace(&mut self._bits_raw[0].0, State::new(nzbw, op));
    }
}

impl fmt::Debug for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bits({:?})", self._bits_raw[0].0)
    }
}
