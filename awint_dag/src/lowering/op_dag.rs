//! This DAG is for lowering into a LUT-only DAG

use std::{
    borrow::Borrow,
    num::NonZeroUsize,
    ops::{Index, IndexMut},
    vec,
};

use awint_ext::{awint_internals::BITS, Bits};
use Op::*;

use crate::{
    lowering::{OpNode, PNode},
    state::next_state_visit_gen,
    triple_arena::Arena,
    EvalError, Op, PState,
};

/// An Operational Directed Acyclic Graph. Contains DAGs of mimicking struct
/// `Op` operations.
#[derive(Debug, Clone)]
pub struct OpDag {
    pub a: Arena<PNode, OpNode<PNode>>,
    /// for keeping nodes alive and having an ordered list for identification
    pub noted: Vec<Option<PNode>>,
    /// A kind of generation counter tracking the highest `visit_num` number
    pub visit_gen: u64,
    /// for capacity reuse in `add_group`
    pub tmp_stack: Vec<(usize, PNode, PState)>,
    /// for capacity reuse in `dec_rc`
    pub tmp_pnode_stack: Vec<PNode>,
}

impl<B: Borrow<PNode>> Index<B> for OpDag {
    type Output = OpNode<PNode>;

    fn index(&self, index: B) -> &OpNode<PNode> {
        self.a.get(*index.borrow()).unwrap()
    }
}

impl<B: Borrow<PNode>> IndexMut<B> for OpDag {
    fn index_mut(&mut self, index: B) -> &mut OpNode<PNode> {
        self.a.get_mut(*index.borrow()).unwrap()
    }
}

impl OpDag {
    /// Constructs a directed acyclic graph from the source trees of `PState`s
    /// from mimicking structs. The optional `note`s should be `Opaque` if
    /// they should remain unmutated through optimizations. The `noted` are
    /// pushed in order to the `OpDag.noted`. If a `noted` is not found in the
    /// source trees of the `leaves` or is optimized away, its entry in
    /// `OpDag.noted` is replaced with a `None`.
    ///
    /// If an error occurs, the DAG (which may be in an unfinished or completely
    /// broken state) is still returned along with the error enum, so that debug
    /// tools like `render_to_svg_file` can be used.
    pub fn new(leaves: &[PState], noted: &[PState]) -> (Self, Result<(), EvalError>) {
        let mut res = Self {
            a: Arena::new(),
            noted: vec![],
            visit_gen: 0,
            tmp_stack: vec![],
            tmp_pnode_stack: vec![],
        };
        let err = res.add_group(leaves, noted, true, 0, None);
        (res, err)
    }

    /// Adds the `leaves` and their source trees to `self`. For each `noted`
    /// node in order, the translated node is pushed to `self.noted` or `None`
    /// is pushed if it is not found in the source trees. If `inc_noted_rc` and
    /// a `noted` node is found, the rc is incremented. The visit numbers of
    /// all the added nodes are set to `self.visit_gen`. Note: the leaves
    /// and all their preceding nodes should not share with previously
    /// existing nodes in this DAG or else there will be duplication.
    pub fn add_group(
        &mut self,
        leaves: &[PState],
        noted: &[PState],
        inc_noted_rc: bool,
        visit: u64,
        mut added: Option<&mut Vec<PNode>>,
    ) -> Result<(), EvalError> {
        // this is for the state side visits not the `visit` for `OpNode`s
        let state_visit = next_state_visit_gen();
        self.tmp_stack.clear();
        for leaf in leaves {
            let leaf_state = leaf.get_state().unwrap();
            if leaf_state.visit != state_visit {
                let p_leaf = self.a.insert(OpNode {
                    nzbw: leaf_state.nzbw,
                    op: Op::Invalid,
                    rc: 0,
                    err: None,
                    location: leaf_state.location,
                    visit,
                });
                leaf.set_state_aux(state_visit, p_leaf);
                if let Some(ref mut v) = added {
                    v.push(p_leaf);
                }
                self.tmp_stack.push((0, p_leaf, *leaf));
                loop {
                    let (current_i, current_p_node, current_p_state) =
                        *self.tmp_stack.last().unwrap();
                    let state = current_p_state.get_state().unwrap();
                    if let Some(t) = Op::translate_root(&state.op) {
                        // reached a root
                        self[current_p_node].op = t;
                        self[current_p_node].nzbw = state.nzbw;
                        self.tmp_stack.pop().unwrap();
                        if let Some((i, ..)) = self.tmp_stack.last_mut() {
                            *i += 1;
                        } else {
                            break
                        }
                    } else if current_i >= state.op.operands_len() {
                        // all operands should be ready
                        self[current_p_node].op =
                            Op::translate(&state.op, |lhs: &mut [PNode], rhs: &[PState]| {
                                for (lhs, rhs) in lhs.iter_mut().zip(rhs.iter()) {
                                    *lhs = rhs.get_state().unwrap().node_map;
                                }
                            });
                        self[current_p_node].nzbw = state.nzbw;
                        self.tmp_stack.pop().unwrap();
                        if let Some((i, ..)) = self.tmp_stack.last_mut() {
                            *i += 1;
                        } else {
                            break
                        }
                    } else {
                        // check next operand
                        let p_next = state.op.operands()[current_i];
                        let next_state = p_next.get_state().unwrap();
                        if next_state.visit == state_visit {
                            // already explored
                            self[next_state.node_map].rc += 1;
                            if let Some((i, ..)) = self.tmp_stack.last_mut() {
                                *i += 1;
                            } else {
                                break
                            }
                        } else {
                            let p = self.a.insert(OpNode {
                                rc: 1,
                                nzbw: state.nzbw,
                                op: Op::Invalid,
                                err: None,
                                location: next_state.location,
                                visit,
                            });
                            p_next.set_state_aux(state_visit, p);
                            if let Some(ref mut v) = added {
                                v.push(p);
                            }
                            self.tmp_stack.push((0, p, state.op.operands()[current_i]));
                        }
                    }
                }
            }
        }
        // handle the noted
        for p_noted in noted {
            let state = p_noted.get_state().unwrap();
            if state.visit == state_visit {
                let p_node = state.node_map;
                if inc_noted_rc {
                    self[p_node].rc += 1;
                }
                self.noted.push(Some(p_node));
            } else {
                self.noted.push(None);
            }
        }
        Ok(())
    }

    pub fn verify_integrity(&mut self) -> Result<(), EvalError> {
        for v in self.a.vals() {
            if let Some(ref err) = v.err {
                return Err(err.clone())
            }
        }
        for p in self.noted.iter().flatten() {
            if self.a.get(*p).is_none() {
                return Err(EvalError::InvalidPtr)
            }
        }
        // TODO there should be more checks
        Ok(())
    }

    /// Assumes `ptr` is a literal
    #[track_caller]
    pub fn lit(&self, ptr: PNode) -> &Bits {
        if let Literal(ref lit) = self[ptr].op {
            lit
        } else {
            panic!("node was not a literal")
        }
    }

    /// Assumes `ptr` is a literal. Returns `None` if the literal does not have
    /// bitwidth 1.
    #[track_caller]
    pub fn bool(&self, ptr: PNode) -> Result<bool, EvalError> {
        if let Literal(ref lit) = self[ptr].op {
            if lit.bw() == 1 {
                Ok(lit.to_bool())
            } else {
                Err(EvalError::WrongBitwidth)
            }
        } else {
            panic!("node was not a literal")
        }
    }

    /// Assumes `ptr` is a literal. Returns `None` if the literal does not have
    /// bitwidth `usize::BITS`.
    #[track_caller]
    pub fn usize(&self, ptr: PNode) -> Result<usize, EvalError> {
        if let Literal(ref lit) = self[ptr].op {
            if lit.bw() == BITS {
                Ok(lit.to_usize())
            } else {
                Err(EvalError::WrongBitwidth)
            }
        } else {
            panic!("node was not a literal")
        }
    }

    pub fn get_bw<B: Borrow<PNode>>(&self, ptr: B) -> NonZeroUsize {
        self[ptr].nzbw
    }

    /// Marks existing node as noted
    pub fn mark_noted(&mut self, p: PNode) -> Option<()> {
        let node = self.a.get_mut(p)?;
        node.rc = node.rc.checked_add(1).unwrap();
        self.noted.push(Some(p));
        Some(())
    }

    /// Decrements the reference count on `p`, and propogating removals if it
    /// goes to zero.
    pub fn dec_rc(&mut self, p: PNode) -> Result<(), EvalError> {
        self[p].rc = if let Some(x) = self[p].rc.checked_sub(1) {
            x
        } else {
            return Err(EvalError::OtherStr("tried to subtract a 0 reference count"))
        };
        if self[p].rc == 0 {
            self.tmp_pnode_stack.clear();
            self.tmp_pnode_stack.push(p);
            while let Some(p) = self.tmp_pnode_stack.pop() {
                let mut delete = false;
                if let Some(node) = self.a.get(p) {
                    if node.rc == 0 {
                        delete = true;
                    }
                }
                if delete {
                    for i in 0..self[p].op.operands_len() {
                        let op = self[p].op.operands()[i];
                        self[op].rc = if let Some(x) = self[op].rc.checked_sub(1) {
                            x
                        } else {
                            return Err(EvalError::OtherStr("tried to subtract a 0 reference count"))
                        };
                        self.tmp_pnode_stack.push(op);
                    }
                    self.a.remove(p).unwrap();
                }
            }
        }
        Ok(())
    }

    /// Forbidden meta pseudo-DSL techniques in which the node at `ptr` is
    /// replaced by a set of lowered `PState` nodes with interfaces `output`
    /// and `operands` corresponding to the original interfaces. Newly added
    /// nodes not including `ptr` are colored with `visit`.
    pub fn graft(
        &mut self,
        ptr: PNode,
        visit: u64,
        output_and_operands: &[PState],
    ) -> Result<(), EvalError> {
        #[cfg(debug_assertions)]
        {
            if (self[ptr].op.operands_len() + 1) != output_and_operands.len() {
                return Err(EvalError::WrongNumberOfOperands)
            }
            for (i, op) in self[ptr].op.operands().iter().enumerate() {
                let current_state = output_and_operands[i + 1].get_state().unwrap();
                if self[op].nzbw != current_state.nzbw {
                    return Err(EvalError::OtherString(format!(
                        "operand {}: a bitwidth of {:?} is trying to be grafted to a bitwidth of \
                         {:?}",
                        i, current_state.nzbw, self[op].nzbw
                    )))
                }
                if !current_state.op.is_opaque() {
                    return Err(EvalError::ExpectedOpaque)
                }
            }
            if self[ptr].nzbw != output_and_operands[0].get_nzbw() {
                return Err(EvalError::WrongBitwidth)
            }
        }
        // note we do not increment the rc because we immediately remove the node's
        // special status
        let err = self.add_group(
            &[output_and_operands[0]],
            output_and_operands,
            false,
            visit,
            None,
        );
        //self.render_to_svg_file(std::path::PathBuf::from("debug.svg"))
        //    .unwrap();
        err?;
        //self.verify_integrity()?;
        let start = self.noted.len() - output_and_operands.len();

        // graft inputs
        for i in 0..(output_and_operands.len() - 1) {
            let grafted = self.noted[start + i + 1];
            let graftee = self[ptr].op.operands()[i];
            if let Some(grafted) = grafted {
                // change the grafted `Opaque` to a `Copy` that routes to the graftee instead of
                // needing to change all the operands of potentially many internal nodes.
                self[grafted].op = Copy([graftee]);
            } else {
                // else the operand is not used because it was optimized away
                self.dec_rc(graftee).unwrap();
            }
        }

        // graft output
        // remove the temporary noted nodes
        let p = self.noted[start].unwrap();
        // this will replace the graftee's location to avoid changing downstream nodes
        let grafted = self.a.remove(p).unwrap();
        let graftee = self.a.replace_and_keep_gen(ptr, grafted).unwrap();

        // preserve original reference count
        self[ptr].rc = graftee.rc;

        // reset the `noted` to its original state
        self.noted.drain(start..);
        Ok(())
    }

    /// Always renders to file, and then returns errors
    #[cfg(feature = "debug")]
    pub fn render_to_svg_file(&mut self, out_file: std::path::PathBuf) -> Result<(), EvalError> {
        let res = self.verify_integrity();
        crate::triple_arena_render::render_to_svg_file(&self.a, false, out_file).unwrap();
        res
    }
}
