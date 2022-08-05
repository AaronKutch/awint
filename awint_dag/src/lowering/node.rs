use std::num::NonZeroUsize;

use triple_arena::Ptr;
#[cfg(feature = "debug")]
use triple_arena_render::{DebugNode, DebugNodeTrait};

use crate::common::{EvalError, Op};

#[derive(Debug)]
pub struct Node<P: Ptr> {
    /// Bitwidth
    pub nzbw: NonZeroUsize,
    /// Operation
    pub op: Op<P>,
    /// Number of dependents
    pub rc: u64,
    pub err: Option<EvalError>,
    /// Used in algorithms to check for visitation
    pub visit: u64,
    /// `Ptr` to self
    pub this_p: P,
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

impl<P: Ptr> Node<P> {}

#[cfg(feature = "debug")]
impl<P: Ptr> DebugNodeTrait<P> for Node<P> {
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
            center: match this.op {
                Op::Literal(ref awi) => vec![format!("{}", awi)],
                Op::StaticLut(_, ref awi) => vec![format!("lut {}", awi)],
                Op::StaticGet(_, inx) => vec![format!("get {}", inx)],
                Op::StaticSet(_, inx) => vec![format!("set {}", inx)],
                _ => vec![this.op.operation_name().to_owned()],
            },
            sinks: vec![],
        };
        if let Some(ref err) = this.err {
            res.center.push(format!("ERROR: {:?}", err));
        }
        res.center.push(format!("{}", this.nzbw));
        if this.this_p == Ptr::invalid() {
            res.center.push("Invalid".to_owned());
        } else {
            res.center.push(format!("{:?}", this.this_p));
        }
        res
    }
}
