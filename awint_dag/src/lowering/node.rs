use std::{num::NonZeroUsize, rc::Rc};

use triple_arena::{Ptr, PtrTrait};
use triple_arena_render::{DebugNode, DebugNodeTrait};

use crate::{mimick, Op};

/// Defines equality using Rc::ptr_eq
#[allow(clippy::derive_hash_xor_eq)] // If `ptr_eq` is true, the `Hash` defined on `Rc` also agrees
#[derive(Debug, Hash, Clone, Eq)]
pub struct PtrEqRc(pub Rc<mimick::State>);

impl PartialEq for PtrEqRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug)]
pub struct Node<P: PtrTrait> {
    /// Bitwidth
    pub nzbw: Option<NonZeroUsize>,
    /// Operation
    pub op: Op,
    /// Operands
    pub ops: Vec<Ptr<P>>,
    /// Dependent nodes that use this one as a source
    pub deps: Vec<Ptr<P>>,
}

/*
impl Hash for Node {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.nzbw.hash(state);
        self.op.hash(state);
        self.ops.hash(state);
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        (self.nzbw == other.nzbw) && (self.op == other.op) && (self.ops == other.ops)
    }
}
*/

impl<P: PtrTrait> DebugNodeTrait<P> for Node<P> {
    fn debug_node(this: &Self) -> DebugNode<P> {
        let names = this.op.operand_names();
        DebugNode {
            sources: this
                .ops
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    (
                        *p,
                        if let Some(s) = names.get(i) {
                            (*s).to_owned()
                        } else {
                            String::new()
                        },
                    )
                })
                .collect(),
            center: vec![this.op.operation_name().to_owned()],
            sinks: vec![],
        }
    }
}
