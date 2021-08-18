use Op::*;

use crate::{
    arena::Ptr,
    lowering::{Dag, EvalError},
    Op,
};

/// I don't expect `deps` to be too long, and some algorithms need `deps` to be
/// in a vector anyway
fn remove(v: &mut Vec<Ptr>, target: Ptr) {
    for i in 0..v.len() {
        if v[i] == target {
            v.swap_remove(i);
            break
        }
    }
}

impl Dag {
    /// Evaluates node, assumed to have an evaluatable operation with all
    /// operands being literals. Note that the DAG may be left in a bad state if
    /// an error is returned.
    pub fn eval_node(&mut self, ptr: Ptr) -> Result<(), EvalError> {
        let op = std::mem::replace(&mut self.dag[ptr].op, Op::Invalid);
        if matches!(op, Literal(_) | Invalid | Opaque) {
            return Err(EvalError::Unevaluatable)
        }
        // check number of operands
        if let Some(expected) = op.operands_len() {
            if self.dag[ptr].ops.len() != expected {
                return Err(EvalError::WrongNumberOfOperands)
            }
        }

        let mut v: Vec<awint_ext::ExtAwi> = vec![];
        for i in 0..self.dag[ptr].ops.len() {
            let input_ptr = self.dag[ptr].ops[i];
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
        if let Some(self_bw) = self.dag[ptr].nzbw {
            let bitwidths: Vec<usize> = v.iter().map(|a| a.nzbw().get()).collect();
            if op.check_bitwidths(self_bw.get(), &bitwidths) {
                return Err(EvalError::WrongBitwidth)
            }
            if op.check_values(self_bw, &v) {
                return Err(EvalError::InvalidOperandValue)
            }
            if let Some(res) = op.eval(self_bw, &v) {
                // remove operand edges
                for op_i in 0..self.dag[ptr].ops.len() {
                    let op = self.dag[ptr].ops[op_i];
                    remove(&mut self.dag[op].deps, ptr);
                    // only if the node is not being used by something else do we remove it
                    if self.dag[op].deps.is_empty() {
                        self.dag.remove(op);
                    }
                }
                self.dag[ptr].ops.clear();
                // make literal
                let _ = std::mem::replace(&mut self.dag[ptr].op, Op::Literal(res));
            } else {
                // some kind of internal bug
                return Err(EvalError::Other)
            }
        }

        Ok(())
    }

    /// Evaluates the DAG as much as is possible
    pub fn eval(&mut self) {
        // evaluatable values
        let list = self.list_ptrs();
        let mut eval: Vec<Ptr> = vec![];
        for p in list {
            if matches!(self.dag[p].op, Literal(_) | Invalid | Opaque) {
                // skip unevaluatable values
                continue
            }
            let mut evaluatable = true;
            for op in &self.dag[p].ops {
                if !matches!(self.dag[op].op, Literal(_)) {
                    evaluatable = false;
                    break
                }
            }
            if evaluatable {
                eval.push(p);
            }
        }

        while let Some(node) = eval.pop() {
            self.eval_node(node).unwrap();
            // check all deps for newly evaluatable nodes
            for dep_i in 0..self.dag[node].deps.len() {
                let dep = self.dag[node].deps[dep_i];
                let mut evaluatable = true;
                for op in &self.dag[dep].ops {
                    if !matches!(self.dag[op].op, Literal(_)) {
                        evaluatable = false;
                        break
                    }
                }
                if evaluatable {
                    eval.push(dep);
                }
            }
        }
    }
}
