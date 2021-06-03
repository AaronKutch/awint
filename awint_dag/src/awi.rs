use alloc::rc::Rc;
use core::num::NonZeroUsize;

use crate::{Bits, Lineage, Op};

#[derive(Debug, Clone)]
pub struct InlAwi(Bits);

impl InlAwi {
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

    #[doc(hidden)]
    pub fn unstable_zero(bw: usize) -> Self {
        Self::new(NonZeroUsize::new(bw).unwrap(), Op::ZeroAssign)
    }

    #[doc(hidden)]
    pub fn unstable_umax(bw: usize) -> Self {
        Self::new(NonZeroUsize::new(bw).unwrap(), Op::UmaxAssign)
    }
}

impl Lineage for InlAwi {
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
