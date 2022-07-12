use triple_arena::{Ptr, PtrTrait};
use Op::*;

use crate::{
    common::{EvalError, Op},
    lowering::Dag,
};

impl<P: PtrTrait> Dag<P> {
    /// Evaluates the node. Assumes the operands are all literals.
    pub fn eval_node(&mut self, ptr: Ptr<P>) -> Result<(), EvalError> {
        if matches!(self[ptr].op, Invalid | Opaque(_) | Literal(_)) {
            return Ok(())
        }
        /*let mut v: Vec<awint_ext::ExtAwi> = vec![];
        for i in 0..self[ptr].ops.len() {
            let input_ptr = self[ptr].ops[i];
            let input = if let Some(node) = self.dag.get(input_ptr) {
                node
            } else {
                return Err(EvalError::InvalidPtr)
            };
            if let Literal(ref lit) = input.op {
                v.push(lit.clone());
            } else {
                return Err(EvalError::NonliteralOperand)
            }
        }

        // check bitwidths and values
        if let Some(self_bw) = self[ptr].nzbw {
            if self[ptr].op.check_bitwidths(self_bw.get()) {
                return Err(EvalError::WrongBitwidth)
            }
            if self[ptr].op.check_values(self_bw, &v) {
                return Err(EvalError::InvalidOperandValue)
            }
            if let Some(res) = self[ptr].op.eval(self_bw, &v) {
                // remove operand edges
                for op_i in 0..self[ptr].ops.len() {
                    let op = self[ptr].ops[op_i];
                    remove(&mut self[op].deps, ptr);
                    // only if the node is not being used by something else do we remove it
                    if self[op].deps.is_empty() {
                        self.dag.remove(op);
                    }
                }
                self[ptr].ops.clear();
                // make literal
                let _ = std::mem::replace(&mut self[ptr].op, Op::Literal(res));
            } else {
                // some kind of internal bug
                return Err(EvalError::OtherStr("eval bug"))
            }
        } else {
            return Err(EvalError::Unevaluatable)
        }*/

        Ok(())
    }

    // FIXME how we accomplish this without backedges is that we start with a leaf
    // as a target and work backwards until reaching roots, keeping a record of
    // the frontier and using counters at the node level

    // Evaluates the DAG as much as is possible
    /*pub fn eval(&mut self) -> Result<(), EvalError> {
        // evaluatable values
        let list = self.ptrs();
        let mut eval: Vec<Ptr<P>> = vec![];
        for p in list {
            if matches!(self[p].op, Invalid | Opaque(_) | Literal(_)) {
                // skip unevaluatable values
                continue
            }
            let mut evaluatable = true;
            for op in self[p].op.operands() {
                if !self[op].op.is_literal() {
                    evaluatable = false;
                    break
                }
            }
            if evaluatable {
                eval.push(p);
            }
        }

        while let Some(node) = eval.pop() {
            if let Err(e) = self.eval_node(node) {
                self[node].err = Some(e.clone());
                return Err(e)
            }
            // check all deps for newly evaluatable nodes
            for dep_i in 0..self[node].deps.len() {
                let dep = self[node].deps[dep_i];
                let mut evaluatable = true;
                for op in &self[dep].ops {
                    if !self[op].op.is_literal() {
                        evaluatable = false;
                        break
                    }
                }
                if evaluatable {
                    eval.push(dep);
                }
            }
        }
        Ok(())
    }*/
}
