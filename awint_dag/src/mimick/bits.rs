use std::{num::NonZeroUsize, rc::Rc};

use crate::{
    mimick::{Lineage, State},
    Op,
};

/// Mimicking `awint_core::Bits`
#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Bits {
    pub(crate) state: Rc<State>,
}

impl Lineage for Bits {
    fn new(nzbw: NonZeroUsize, op: Op, ops: Vec<Rc<State>>) -> Self {
        Self {
            state: Rc::new(State { nzbw, op, ops }),
        }
    }

    fn state(&self) -> Rc<State> {
        Rc::clone(&self.state)
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
        self.state.nzbw
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
}
