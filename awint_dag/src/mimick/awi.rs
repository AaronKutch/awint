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
        assert_inlawi_invariants::<BW, LEN>();
        assert_inlawi_invariants_slice::<BW, LEN>(raw);
        // `collect` does not work
        let mut v = Vec::new();
        for x in raw.iter() {
            v.push(*x);
        }
        Self::new(NonZeroUsize::new(BW).unwrap(), Op::LitRawSliceAssign(v))
    }

    pub fn zero() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(NonZeroUsize::new(BW).unwrap(), Op::ZeroAssign)
    }

    pub fn umax() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(NonZeroUsize::new(BW).unwrap(), Op::UmaxAssign)
    }

    pub fn imax() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(NonZeroUsize::new(BW).unwrap(), Op::ImaxAssign)
    }

    pub fn imin() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(NonZeroUsize::new(BW).unwrap(), Op::IminAssign)
    }

    pub fn uone() -> Self {
        assert_inlawi_invariants::<BW, LEN>();
        Self::new(NonZeroUsize::new(BW).unwrap(), Op::UoneAssign)
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

    pub fn zero(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::ZeroAssign)
    }

    pub fn umax(bw: NonZeroUsize) -> Self {
        Self::new(bw, Op::UmaxAssign)
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
