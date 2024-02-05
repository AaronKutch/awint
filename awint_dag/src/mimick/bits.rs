use std::{
    fmt,
    marker::PhantomData,
    num::NonZeroUsize,
    ptr::{self},
    rc::Rc,
};

use crate::{
    common::EAwi,
    mimick::{Awi, ExtAwi, InlAwi},
    EvalResult, Lineage, Op, PState,
};

// this is a workaround for https://github.com/rust-lang/rust/issues/57749 that works on stable

/// Mimicking [awint_ext::Bits]
#[repr(C)] // needed for `internal_as_ref*`
pub struct Bits {
    _no_send_or_sync: PhantomData<fn() -> Rc<()>>,
    // use different names for the different raw `PState`s, or else Rust can think we are
    // trying to go through the `Deref` impls
    _state: PState,
    _dst: [()],
}

// Safety: `Bits` follows standard slice initialization invariants and is marked
// `#[repr(transparent)]`. The explicit lifetimes make sure they do not become
// unbounded.

impl<'a> ExtAwi {
    pub(in crate::mimick) fn internal_as_ref(&'a self) -> &'a Bits {
        // Safety: `ExtAwi` is a `#[repr(C)]` `PState`, and here we use a pointer from
        // `self` to create `Bits` with a zero length `_dst`. The explicit lifetimes
        // make sure they do not become unbounded.
        let bits = ptr::slice_from_raw_parts(self as *const Self, 0) as *const Bits;
        unsafe { &*bits }
    }

    pub(in crate::mimick) fn internal_as_mut(&'a mut self) -> &'a mut Bits {
        // Safety: `ExtAwi` is a `#[repr(C)]` `PState`, and here we use a pointer from
        // `self` to create `Bits` with a zero length `_dst`. The explicit lifetimes
        // make sure they do not become unbounded.
        let bits = ptr::slice_from_raw_parts_mut(self as *mut Self, 0) as *mut Bits;
        unsafe { &mut *bits }
    }
}

impl<'a> Awi {
    pub(in crate::mimick) fn internal_as_ref(&'a self) -> &'a Bits {
        // Safety: `Awi` is a `#[repr(C)]` `PState`, and here we use a pointer from
        // `self` to create `Bits` with a zero length `_dst`. The explicit lifetimes
        // make sure they do not become unbounded.
        let bits = ptr::slice_from_raw_parts(self as *const Self, 0) as *const Bits;
        unsafe { &*bits }
    }

    pub(in crate::mimick) fn internal_as_mut(&'a mut self) -> &'a mut Bits {
        // Safety: `Awi` is a `#[repr(C)]` `PState`, and here we use a pointer from
        // `self` to create `Bits` with a zero length `_dst`. The explicit lifetimes
        // make sure they do not become unbounded.
        let bits = ptr::slice_from_raw_parts_mut(self as *mut Self, 0) as *mut Bits;
        unsafe { &mut *bits }
    }
}

impl<'a, const BW: usize, const LEN: usize> InlAwi<BW, LEN> {
    pub(in crate::mimick) fn internal_as_ref(&'a self) -> &'a Bits {
        // Safety: `InlAwi` is a `#[repr(C)]` `PState`, and here we use a pointer from
        // `self` to create `Bits` with a zero length `_dst`. The explicit lifetimes
        // make sure they do not become unbounded.
        let bits = ptr::slice_from_raw_parts(self as *const Self, 0) as *const Bits;
        unsafe { &*bits }
    }

    pub(in crate::mimick) fn internal_as_mut(&'a mut self) -> &'a mut Bits {
        // Safety: `InlAwi` is a `#[repr(C)]` `PState`, and here we use a pointer from
        // `self` to create `Bits` with a zero length `_dst`. The explicit lifetimes
        // make sure they do not become unbounded.
        let bits = ptr::slice_from_raw_parts_mut(self as *mut Self, 0) as *mut Bits;
        unsafe { &mut *bits }
    }
}

impl Lineage for &Bits {
    fn state(&self) -> PState {
        self._state
    }
}

impl Lineage for &mut Bits {
    fn state(&self) -> PState {
        self._state
    }
}

impl Bits {
    pub fn nzbw(&self) -> NonZeroUsize {
        self.state_nzbw()
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

    /// Not intended for most users.
    pub fn set_state(&mut self, state: PState) {
        // other `PState`s that need the old state will keep it alive despite the
        // previous one being dropped
        self._state = state;
    }

    /// Not intended for most users.
    ///
    /// This function is guaranteed to not return `Option::Opaque`, and may
    /// return `Option::Some` in cases that need external checking
    #[track_caller]
    #[must_use]
    pub fn update_state(
        &mut self,
        nzbw: NonZeroUsize,
        p_state_op: Op<PState>,
    ) -> crate::mimick::Option<()> {
        // Eager evaluation, currently required because in the macros we had to pass
        // `Into<dag::usize>` into panicking constructor functions, and we need to know
        // the bitwidths at that time. Also, it turns out we may want eager evaluation
        // also because of early `Noop` and `Pass` evaluation results that can be caught
        // early.
        let lit_op: Op<EAwi> = Op::translate(&p_state_op, |lhs: &mut [EAwi], rhs: &[PState]| {
            for (lhs, rhs) in lhs.iter_mut().zip(rhs.iter()) {
                if let Op::Literal(ref lit) = rhs.get_op() {
                    *lhs = EAwi::KnownAwi(lit.clone());
                } else {
                    *lhs = EAwi::Bitwidth(rhs.get_nzbw())
                }
            }
        });
        match lit_op.eval(nzbw) {
            EvalResult::Valid(x) => {
                self.set_state(PState::new(x.nzbw(), Op::Literal(x), None));
                crate::mimick::Option::Some(())
            }
            EvalResult::Pass(x) => {
                self.set_state(PState::new(x.nzbw(), Op::Literal(x), None));
                crate::mimick::Option::None
            }
            EvalResult::PassUnevaluatable => {
                self.set_state(PState::new(nzbw, p_state_op, None));
                crate::mimick::Option::None
            }
            EvalResult::Noop => {
                // do not update state
                crate::mimick::Option::None
            }
            EvalResult::Unevaluatable => {
                self.set_state(PState::new(nzbw, p_state_op, None));
                crate::mimick::Option::Some(())
            }
            EvalResult::AssertionSuccess => {
                self.set_state(PState::new(nzbw, p_state_op, None));
                crate::mimick::Option::Some(())
            }
            EvalResult::AssertionFailure => {
                panic!(
                    "assertion failure from `awint_dag` mimicking type eager evaluation {nzbw} \
                     {self:?} {p_state_op:?}"
                );
            }
            EvalResult::Error(e) => {
                panic!("{e:?}");
            }
        }
    }

    #[must_use]
    pub fn copy_(&mut self, rhs: &Self) -> crate::mimick::Option<()> {
        // directly use the state of `rhs`
        if self.bw() == rhs.bw() {
            self.set_state(rhs.state());
            crate::mimick::Option::Some(())
        } else {
            crate::mimick::Option::None
        }
    }
}

impl fmt::Debug for Bits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bits({:?})", self.state())
    }
}

impl AsRef<Bits> for &Bits {
    fn as_ref(&self) -> &Bits {
        self
    }
}
