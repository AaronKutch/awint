use std::{num::NonZeroUsize, rc::Rc};

use awint_internals::*;

use crate::mimick::{Bits, Lineage, Op};

/// Mimicking `awint_core::InlAwi`
#[derive(Debug, Clone)]
pub struct InlAwi<const BW: usize, const LEN: usize>(Bits);

impl<const BW: usize, const LEN: usize> InlAwi<BW, LEN> {
    pub(crate) fn new(bw: NonZeroUsize, op: Op) -> Self {
        // double check the invariants
        assert_inlawi_invariants::<BW, LEN>();
        Self(Bits::new(bw, op))
    }

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

    #[doc(hidden)]
    pub fn unstable_from_slice(raw: &[usize]) -> Self {
        Self::new(
            NonZeroUsize::new(BW).unwrap(),
            Op::LitAssign(awint_ext::ExtAwi::from_bits(
                awint_core::InlAwi::<BW, LEN>::unstable_from_slice(raw).const_as_ref(),
            )),
        )
    }

    pub fn zero() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        let nzbw = NonZeroUsize::new(BW).unwrap();
        Self::new(nzbw, Op::ZeroAssign(nzbw))
    }

    pub fn umax() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        let nzbw = NonZeroUsize::new(BW).unwrap();
        Self::new(nzbw, Op::UmaxAssign(nzbw))
    }

    pub fn imax() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        let nzbw = NonZeroUsize::new(BW).unwrap();
        Self::new(nzbw, Op::ImaxAssign(nzbw))
    }

    pub fn imin() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        let nzbw = NonZeroUsize::new(BW).unwrap();
        Self::new(nzbw, Op::IminAssign(nzbw))
    }

    pub fn uone() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        let nzbw = NonZeroUsize::new(BW).unwrap();
        Self::new(nzbw, Op::UoneAssign(nzbw))
    }
}

impl<const BW: usize, const LEN: usize> Lineage for InlAwi<BW, LEN> {
    fn nzbw(&self) -> NonZeroUsize {
        self.0.nzbw()
    }

    fn op(&self) -> Rc<Op> {
        self.0.op()
    }

    fn op_mut(&mut self) -> &mut Rc<Op> {
        self.0.op_mut()
    }
}

/// Mimicking `awint_ext::ExtAwi`
#[derive(Debug, Clone)]
pub struct ExtAwi(Bits);

impl ExtAwi {
    pub(crate) fn new(bw: NonZeroUsize, op: Op) -> Self {
        Self(Bits::new(bw, op))
    }

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
        let mut tmp = Self::new(bits.nzbw(), Op::ZeroAssign(bits.nzbw()));
        tmp.const_as_mut().copy_assign(bits).unwrap();
        tmp
    }

    pub fn zero(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::ZeroAssign(bw))
    }

    pub fn umax(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::UmaxAssign(bw))
    }

    pub fn imax(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::ImaxAssign(bw))
    }

    pub fn imin(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::IminAssign(bw))
    }

    pub fn uone(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::UoneAssign(bw))
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

impl Lineage for ExtAwi {
    fn nzbw(&self) -> NonZeroUsize {
        self.0.nzbw()
    }

    fn op(&self) -> Rc<Op> {
        self.0.op()
    }

    fn op_mut(&mut self) -> &mut Rc<Op> {
        self.0.op_mut()
    }
}
