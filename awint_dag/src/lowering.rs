mod dag;
mod eval;

use std::{num::NonZeroUsize, rc::Rc};

use triple_arena::prelude::*;
use triple_arena_render::*;

use crate::{mimick, Op};

ptr_trait_struct_with_gen!(P0);

/// Defines equality using Rc::ptr_eq
#[allow(clippy::derive_hash_xor_eq)] // If `ptr_eq` is true, the `Hash` defined on `Rc` also agrees
#[derive(Debug, Hash, Clone, Eq)]
pub struct PtrEqRc(Rc<mimick::State>);

impl PartialEq for PtrEqRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Debug)]
pub struct Node {
    /// Bitwidth
    pub nzbw: Option<NonZeroUsize>,
    /// Operation
    pub op: Op,
    /// Operands
    pub ops: Vec<Ptr<P0>>,
    /// Dependent nodes that use this one as a source
    pub deps: Vec<Ptr<P0>>,
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

impl DebugNodeTrait<P0> for Node {
    fn debug_node(this: &Self) -> DebugNode<P0> {
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

#[derive(Debug)]
pub struct Dag {
    pub dag: Arena<P0, Node>,
}

#[derive(Debug)]
pub enum EvalError {
    // Thrown if a `Literal`, `Invalid`, or `Opaque` node is attempted to be evaluated
    Unevaluatable,
    // wrong number of operands
    WrongNumberOfOperands,
    // An operand points nowhere, so the DAG is broken
    InvalidPtr,
    // an operand is not a `Literal`
    NonliteralOperand,
    // wrong bitwidths of operands
    WrongBitwidth,
    // wrong integer value of an operand, such as overshifting from a shift operation or going out
    // of bounds in a field operation
    InvalidOperandValue,
    // Some other kind of brokenness, such as dependency edges not agreeing with operand edges
    Other,
}
