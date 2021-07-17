mod arena;
mod dag;

use std::{num::NonZeroUsize, rc::Rc};

pub use arena::{Arena, Ptr};

use crate::{mimick, Op};

/// Defines equality using Rc::ptr_eq
#[allow(clippy::derive_hash_xor_eq)] // If `ptr_eq` is true, the `Hash` defined on `Rc` also agrees
#[derive(Debug, Hash, Clone, Eq)]
pub struct PtrEqRc(Rc<mimick::State>);

impl PartialEq for PtrEqRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Node {
    /// Bitwidth
    pub nzbw: NonZeroUsize,
    /// Operation
    pub op: Op,
    /// Operands
    pub ops: Vec<Ptr>,
    /// Dependent nodes that use this one as a source
    pub deps: Vec<Ptr>,
}

#[derive(Debug)]
pub struct Dag {
    dag: Arena<Node>,
}
