mod arena;
mod dag;
mod op;

use std::{num::NonZeroUsize, rc::Rc};

pub use arena::{Arena, Ptr};
pub use op::Op;

use crate::mimick;

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Node {
    bw: Option<NonZeroUsize>,
    op: Op,
    num_unlowered_sources: usize,
    /// Dependent nodes that use this one as a source
    deps: Vec<Ptr>,
}

impl Node {
    pub fn new(op: Op) -> Self {
        Node {
            bw: None,
            op,
            num_unlowered_sources: 0,
            deps: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Dag {
    dag: Arena<Node>,
}

/// Defines equality using Rc::ptr_eq
#[allow(clippy::derive_hash_xor_eq)] // If `ptr_eq` is true, the `Hash` defined on `Rc` also agrees
#[derive(Debug, Hash, Clone, Eq)]
pub struct PtrEqRc(Rc<mimick::Op>);

impl PartialEq for PtrEqRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
