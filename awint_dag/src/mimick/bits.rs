use std::{fmt, marker::PhantomData, mem, num::NonZeroUsize, ptr, rc::Rc};

use awint_ext::awi;

use crate::{
    mimick::{ExtAwi, InlAwi},
    EvalError, EvalResult, Lineage, Op, PState,
};

// this is a workaround for https://github.com/rust-lang/rust/issues/57749 that works on stable
// TODO fix when PR 83850 is merged

/// Mimicking `awint_core::Bits`
#[repr(transparent)] // for the transmute
pub struct Bits {
    _no_send_or_sync: PhantomData<Rc<()>>,
    // use different names for the different raw `PState`s, or else Rust can think we are
    // trying to go through the `Deref` impls
    _bits_raw: [PState],
}

// Safety: `Bits` follows standard slice initialization invariants and is marked
// `#[repr(transparent)]`. The explicit lifetimes make sure they do not become
// unbounded.

impl<'a> Bits {
    /// Assumes this is called on a pointer from a `[PState; 1]`
    unsafe fn from_raw_parts(raw_ptr: *const PState) -> &'a Self {
        unsafe { mem::transmute::<&[PState], &Bits>(&*ptr::slice_from_raw_parts(raw_ptr, 1)) }
    }

    /// Assumes this is called on a pointer from a `[PState; 1]`
    unsafe fn from_raw_parts_mut(raw_ptr: *mut PState) -> &'a mut Self {
        unsafe {
            mem::transmute::<&mut [PState], &mut Bits>(&mut *ptr::slice_from_raw_parts_mut(
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
    fn state(&self) -> PState {
        self._bits_raw[0]
    }
}

impl Lineage for &mut Bits {
    fn state(&self) -> PState {
        self._bits_raw[0]
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

    pub(crate) fn set_state(&mut self, state: PState) {
        // other `PState`s that need the old state will keep it alive despite this one
        // being dropped
        let _: PState = mem::replace(&mut self._bits_raw[0], state);
    }

    /// This function is guaranteed to not return `Option::Opaque`, and may
    /// return `Option::Some` in cases that need external checking
    #[track_caller]
    #[must_use]
    pub(crate) fn update_state(
        &mut self,
        nzbw: NonZeroUsize,
        p_state_op: Op<PState>,
    ) -> crate::mimick::Option<()> {
        // eager evaluation, currently required because in the macros we had to pass
        // `Into<dag::usize>` into panicking constructor functions, and we need to know
        // the bitwidths at that time
        let mut all_literals = true;
        for p_state in p_state_op.operands() {
            if !p_state.get_state().unwrap().op.is_literal() {
                all_literals = false;
                break
            }
        }
        if all_literals {
            let lit_op: Op<awi::ExtAwi> =
                Op::translate(&p_state_op, |lhs: &mut [awi::ExtAwi], rhs: &[PState]| {
                    for (lhs, rhs) in lhs.iter_mut().zip(rhs.iter()) {
                        if let Op::Literal(ref lit) = rhs.get_state().unwrap().op {
                            *lhs = lit.clone();
                        } else {
                            unreachable!()
                        }
                    }
                });
            match lit_op.eval(nzbw) {
                EvalResult::Valid(x) => {
                    self.set_state(PState::new(x.nzbw(), Op::Literal(x)));
                    crate::mimick::Option::Some(())
                }
                EvalResult::Pass(x) => {
                    self.set_state(PState::new(x.nzbw(), Op::Literal(x)));
                    crate::mimick::Option::None
                }
                EvalResult::Noop => {
                    // do not update state
                    crate::mimick::Option::None
                }
                EvalResult::Error(e) => {
                    if matches!(e, EvalError::Unevaluatable) {
                        self.set_state(PState::new(nzbw, p_state_op));
                        crate::mimick::Option::Some(())
                    } else {
                        panic!("{e:?}");
                    }
                }
            }
        } else {
            self.set_state(PState::new(nzbw, p_state_op));
            crate::mimick::Option::Some(())
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
        write!(f, "Bits({:?})", self._bits_raw[0])
    }
}

impl AsRef<Bits> for &Bits {
    fn as_ref(&self) -> &Bits {
        self
    }
}
