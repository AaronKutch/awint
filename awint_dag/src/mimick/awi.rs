use std::{num::NonZeroUsize, rc::Rc};

use awint_internals::*;

use crate::{
    mimick::{Bits, ConstBwLineage, Lineage, State},
    Op,
};

/// Mimicking `awint_core::InlAwi`
#[derive(Debug)]
pub struct InlAwi<const BW: usize, const LEN: usize>(Bits);

impl<const BW: usize, const LEN: usize> ConstBwLineage for InlAwi<BW, LEN> {
    fn new(op: Op, ops: Vec<Rc<State>>) -> Self {
        Self(Bits::new(Self::const_nzbw(), op, ops))
    }

    fn hidden_const_nzbw() -> NonZeroUsize {
        Self::const_nzbw()
    }

    fn state(&self) -> Rc<State> {
        self.0.state()
    }
}

impl<const BW: usize, const LEN: usize> Clone for InlAwi<BW, LEN> {
    fn clone(&self) -> Self {
        Self::new(Op::CopyAssign, vec![self.state()])
    }
}

impl<const BW: usize, const LEN: usize> InlAwi<BW, LEN> {
    pub fn const_nzbw() -> NonZeroUsize {
        NonZeroUsize::new(BW).unwrap()
    }

    pub fn const_bw() -> usize {
        BW
    }

    pub fn nzbw(&self) -> NonZeroUsize {
        Self::const_nzbw()
    }

    pub fn bw(&self) -> usize {
        Self::const_bw()
    }

    pub fn const_as_ref(&self) -> &Bits {
        &self.0
    }

    pub fn const_as_mut(&mut self) -> &mut Bits {
        &mut self.0
    }

    #[doc(hidden)]
    pub fn unstable_from_slice(raw: &[usize]) -> Self {
        Self::new(
            Op::Literal(awint_ext::ExtAwi::from_bits(
                awint_core::InlAwi::<BW, LEN>::unstable_from_slice(raw).const_as_ref(),
            )),
            vec![],
        )
    }

    pub fn zero() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::ZeroAssign, vec![])
    }

    pub fn umax() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::UmaxAssign, vec![])
    }

    pub fn imax() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::ImaxAssign, vec![])
    }

    pub fn imin() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::IminAssign, vec![])
    }

    pub fn uone() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(Op::UoneAssign, vec![])
    }
}

/// Mimicking `awint_ext::ExtAwi`
#[derive(Debug)]
pub struct ExtAwi(Bits);

impl Lineage for ExtAwi {
    fn new(bw: NonZeroUsize, op: Op, ops: Vec<Rc<State>>) -> Self {
        Self(Bits::new(bw, op, ops))
    }

    fn state(&self) -> Rc<State> {
        self.0.state()
    }
}

impl Clone for ExtAwi {
    fn clone(&self) -> Self {
        Self::new(self.nzbw(), Op::CopyAssign, vec![self.state()])
    }
}

impl ExtAwi {
    /*
    pub fn bw(&self) -> prim::usize {
        prim::usize::new(BwAssign, vec![self.state()])
    }
    */

    pub fn nzbw(&self) -> NonZeroUsize {
        self.0.nzbw()
    }

    pub fn bw(&self) -> usize {
        self.0.bw()
    }

    pub fn const_as_ref(&self) -> &Bits {
        &self.0
    }

    pub fn const_as_mut(&mut self) -> &mut Bits {
        &mut self.0
    }

    pub fn from_bits(bits: &Bits) -> ExtAwi {
        Self::new(bits.nzbw(), Op::CopyAssign, vec![bits.state()])
    }

    pub fn zero(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::ZeroAssign, vec![])
    }

    pub fn umax(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::UmaxAssign, vec![])
    }

    pub fn imax(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::ImaxAssign, vec![])
    }

    pub fn imin(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::IminAssign, vec![])
    }

    pub fn uone(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::UoneAssign, vec![])
    }

    #[doc(hidden)]
    pub fn panicking_zero(bw: usize) -> Self {
        Self::zero(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_umax(bw: usize) -> Self {
        Self::umax(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_imax(bw: usize) -> Self {
        Self::imax(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_imin(bw: usize) -> Self {
        Self::imin(NonZeroUsize::new(bw).unwrap())
    }

    #[doc(hidden)]
    pub fn panicking_uone(bw: usize) -> Self {
        Self::uone(NonZeroUsize::new(bw).unwrap())
    }
}
