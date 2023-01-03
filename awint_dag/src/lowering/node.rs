use std::num::NonZeroUsize;

#[cfg(feature = "debug")]
use crate::triple_arena_render::{DebugNode, DebugNodeTrait};
use crate::{common::DummyDefault, triple_arena::Ptr, EvalError, Op};

/// An Operational Node for an `OpDag` that includes the operation and other
/// data used in algorithms
#[derive(Debug, Clone)]
pub struct OpNode<P: Ptr + DummyDefault> {
    /// Bitwidth
    pub nzbw: NonZeroUsize,
    /// Operation
    pub op: Op<P>,
    /// Number of dependents
    pub rc: u64,
    pub err: Option<EvalError>,
    /// Used in algorithms to check for visitation
    pub visit: u64,
}

#[cfg(feature = "debug")]
impl<P: Ptr + DummyDefault> DebugNodeTrait<P> for OpNode<P> {
    fn debug_node(p_this: P, this: &Self) -> DebugNode<P> {
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
                Op::Opaque(_) => vec![format!("opaque {}", this.nzbw)],
                Op::Literal(ref awi) => vec![format!("{awi}")],
                Op::StaticLut(_, ref awi) => vec![format!("lut {awi}")],
                Op::StaticGet(_, inx) => vec![format!("get {inx}")],
                Op::StaticSet(_, inx) => vec![format!("set {inx}")],
                _ => vec![this.op.operation_name().to_owned()],
            },
            sinks: vec![],
        };
        if let Some(ref err) = this.err {
            res.center.push(format!("ERROR: {err:?}"));
        }
        res.center.push(format!("{} - {}", this.nzbw, this.rc));
        res.center.push(format!("{p_this:?}"));
        res
    }
}
