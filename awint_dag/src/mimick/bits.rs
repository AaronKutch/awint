use std::{num::NonZeroUsize, rc::Rc};

use crate::{
    mimick::{Lineage, State},
    Op,
};

/// Mimicking `awint_core::Bits`
#[derive(Debug)]
pub struct Bits {
    pub(crate) state: Rc<State>,
}

impl Lineage for Bits {
    fn from_state(state: Rc<State>) -> Self {
        Self { state }
    }

    fn hidden_const_nzbw() -> Option<NonZeroUsize> {
        None
    }

    fn state(&self) -> Rc<State> {
        Rc::clone(&self.state)
    }
}

impl Bits {
    pub(crate) fn new(nzbw: NonZeroUsize, op: Op, ops: Vec<Rc<State>>) -> Self {
        Self {
            state: State::new(Some(nzbw), op, ops),
        }
    }

    // TODO if we use dynamic bitwidths do we do something like this?
    /*
    pub fn bw(&self) -> prim::usize {
        prim::usize::new(BwAssign, vec![self.state()])
    }
    */

    pub fn nzbw(&self) -> NonZeroUsize {
        self.state.nzbw.unwrap()
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
