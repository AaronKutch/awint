use std::{num::NonZeroUsize, rc::Rc};

use triple_arena::{ptr_trait_struct_with_gen, Ptr, PtrTrait};
#[cfg(feature = "debug")]
use triple_arena_render::{DebugNode, DebugNodeTrait};

use crate::common::{EvalError, Op, State};

// used in some internal algorithms
ptr_trait_struct_with_gen!(P0);

/// Defines equality using Rc::ptr_eq
#[allow(clippy::derive_hash_xor_eq)] // If `ptr_eq` is true, the `Hash` defined on `Rc` also agrees
#[derive(Debug, Hash, Clone, Eq)]
pub struct PtrEqRc(pub Rc<State>);

impl PartialEq for PtrEqRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug, Default)]
pub struct Node<P: PtrTrait> {
    /// Bitwidth
    pub nzbw: Option<NonZeroUsize>,
    /// Operation
    pub op: Op<Ptr<P>>,
    /// Number of dependents
    pub rc: u64,
    pub err: Option<EvalError>,
    /// Used in algorithms to check for visitation
    pub visit_num: u64,
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

impl<P: PtrTrait> Node<P> {}

#[cfg(feature = "debug")]
impl<P: PtrTrait> DebugNodeTrait<P> for Node<P> {
    fn debug_node(this: &Self) -> DebugNode<P> {
        let names = this.op.operand_names();
        let mut res = DebugNode {
            sources: this
                .op
                .operands()
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    (
                        *p,
                        if names.len() > 1 {
                            names[i].to_owned()
                        } else {
                            String::new()
                        },
                    )
                })
                .collect(),
            center: if let Op::Literal(ref awi) = this.op {
                vec![format!("{}", awi)]
            } else {
                vec![this.op.operation_name().to_owned()]
            },
            sinks: vec![],
        };
        if let Some(ref err) = this.err {
            res.center.push(format!("ERROR: {:?}", err));
        }
        if let Some(w) = this.nzbw {
            res.center.push(format!("{}", w));
        }
        //res.center.push(format!("rc: {}", this.rc));
        res
    }
}
