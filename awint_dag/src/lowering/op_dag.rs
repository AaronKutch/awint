//! This DAG is for lowering into a LUT-only DAG

use std::{
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
    num::{NonZeroU64, NonZeroUsize},
    ops::{Index, IndexMut},
    vec,
};

use awint_ext::{awint_internals::USIZE_BITS, Bits};
use Op::*;

use crate::{
    basic_state_epoch::StateEpoch,
    lowering::{OpNode, PNode},
    triple_arena::Arena,
    EvalError, Op, PNote, PState,
};

/// An Operational Directed Acyclic Graph. Contains DAGs of mimicking struct
/// `Op` operations.
#[derive(Debug, Clone)]
pub struct OpDag {
    pub a: Arena<PNode, OpNode<PNode>>,
    /// for keeping nodes alive and identified
    pub note_arena: Arena<PNote, PNode>,
    /// Assertions stored in the arena
    pub assertions: Vec<PNote>,
    /// A kind of generation counter tracking the highest `visit_num` number
    pub visit_gen: u64,
    /// for capacity reuse in `add_group`
    pub tmp_stack: Vec<(usize, PNode, PState)>,
    /// for capacity reuse in `dec_rc`
    pub tmp_pnode_stack: Vec<PNode>,
    pub tmp_pnodes_added: Vec<PNode>,
}

impl<B: Borrow<PNode>> Index<B> for OpDag {
    type Output = OpNode<PNode>;

    fn index(&self, index: B) -> &OpNode<PNode> {
        self.a.get(*index.borrow()).expect("PNode not found")
    }
}

impl<B: Borrow<PNode>> IndexMut<B> for OpDag {
    fn index_mut(&mut self, index: B) -> &mut OpNode<PNode> {
        self.a.get_mut(*index.borrow()).expect("PNode not found")
    }
}

impl OpDag {
    pub fn new() -> Self {
        Self {
            a: Arena::new(),
            note_arena: Arena::new(),
            assertions: vec![],
            visit_gen: 0,
            tmp_stack: vec![],
            tmp_pnode_stack: vec![],
            tmp_pnodes_added: vec![],
        }
    }

    /// Constructs a directed acyclic graph from the `PState`s of mimicking
    /// structs associated with `epoch`. Assertions associated with the `epoch`
    /// are automatically noted.
    ///
    /// If an error occurs, the DAG (which may be in an unfinished or completely
    /// broken state) is still returned along with the error enum, so that debug
    /// tools like `render_to_svg_file` can be used.
    pub fn from_epoch(epoch: &StateEpoch) -> (Self, Result<(), EvalError>) {
        let mut res = Self::new();
        let state_visit = epoch.next_visit_gen();
        let err = res.add_chain(epoch, epoch.latest_state(), state_visit, 0, false);
        for assertion in epoch.assertions().states() {
            let p_note = res.note_pstate(epoch, assertion).unwrap();
            res.assertions.push(p_note);
        }
        (res, err)
    }

    #[must_use]
    pub fn pnote_get_node(&mut self, p_note: PNote) -> Option<&OpNode<PNode>> {
        let p = self.note_arena.get(p_note)?;
        if let Some(node) = self.a.get(*p) {
            Some(node)
        } else {
            None
        }
    }

    #[must_use]
    pub fn pnote_get_mut_node(&mut self, p_note: PNote) -> Option<&mut OpNode<PNode>> {
        let p = self.note_arena.get(p_note)?;
        if let Some(node) = self.a.get_mut(*p) {
            Some(node)
        } else {
            None
        }
    }

    /// Get a `PNote` that will act as a stable reference to the `PNode` version
    /// of `p_state`. Returns `None` if the mapping cannot be found.
    #[must_use]
    pub fn note_pstate(&mut self, epoch: &StateEpoch, p_state: PState) -> Option<PNote> {
        self.note_pnode(epoch.pstate_to_pnode(p_state))
    }

    /// Marks existing node as noted
    #[must_use]
    pub fn note_pnode(&mut self, p: PNode) -> Option<PNote> {
        let node = self.a.get_mut(p)?;
        node.rc = node.rc.checked_add(1).unwrap();
        Some(self.note_arena.insert(p))
    }

    /// Unmarks noted node
    pub fn unnote_pnote(&mut self, p: PNote) -> Option<PNode> {
        let p_node = self.note_arena.remove(p)?;
        self.dec_rc(p_node).unwrap();
        Some(p_node)
    }

    /// Developer function that adds all states in the thread local state list
    /// starting with `latest_state` to `self`. If `record_added` then all added
    /// nodes are pushed to `self.tmp_pnodes_added`
    pub fn add_chain(
        &mut self,
        epoch: &StateEpoch,
        latest_state: Option<PState>,
        state_visit: NonZeroU64,
        node_visit: u64,
        record_added: bool,
    ) -> Result<(), EvalError> {
        self.tmp_stack.clear();
        let mut loop_pstate = latest_state;
        while let Some(leaf_pstate) = loop_pstate {
            let mut continue_loop = false;
            epoch.get_mut_state(leaf_pstate, |leaf| {
                if leaf.visit == state_visit {
                    loop_pstate = leaf.prev_in_epoch;
                    continue_loop = true;
                } else {
                    let p_leaf = self.a.insert(OpNode {
                        nzbw: leaf.nzbw,
                        op: Op::Invalid,
                        rc: 0,
                        err: None,
                        location: leaf.location,
                        visit: node_visit,
                    });
                    leaf.node_map = p_leaf;
                    if record_added {
                        self.tmp_pnodes_added.push(p_leaf);
                    }
                    self.tmp_stack.push((0, p_leaf, leaf_pstate));
                    loop_pstate = leaf.prev_in_epoch;
                }
            });
            if continue_loop {
                continue
            }
            // begin DFS proper
            loop {
                let (current_i, current_p_node, current_p_state) = *self.tmp_stack.last().unwrap();
                let mut break_dfs = false;
                let mut check_next_op = None;
                let mut all_operands_ready = None;
                epoch.get_mut_state(current_p_state, |state| {
                    if let Some(t) = Op::translate_root(&state.op) {
                        // reached a root
                        self[current_p_node].op = t;
                        self.tmp_stack.pop().unwrap();
                        if let Some((i, ..)) = self.tmp_stack.last_mut() {
                            *i += 1;
                        } else {
                            break_dfs = true;
                        }
                    } else if current_i >= state.op.operands_len() {
                        // all operands should be ready
                        all_operands_ready = Some(state.op.clone());
                    } else {
                        // check next operand
                        check_next_op = Some(state.op.operands()[current_i]);
                    }
                });
                if let Some(op) = all_operands_ready {
                    // need this separate because `pstate_to_pnode` borrows the same thread local
                    // data as `get_mut_state`
                    self[current_p_node].op =
                        Op::translate(&op, |lhs: &mut [PNode], rhs: &[PState]| {
                            for (lhs, rhs) in lhs.iter_mut().zip(rhs.iter()) {
                                *lhs = epoch.pstate_to_pnode(*rhs);
                            }
                        });
                    // TODO this is a hack for starlight `Loop`s
                    if matches!(self[current_p_node].op, Op::Opaque(_, Some("LoopHandle"))) {
                        self[current_p_node].rc += 1;
                    }
                    self.tmp_stack.pop().unwrap();
                    if let Some((i, ..)) = self.tmp_stack.last_mut() {
                        *i += 1;
                    } else {
                        break_dfs = true;
                    }
                }
                if let Some(p_next) = check_next_op {
                    epoch.get_mut_state(p_next, |next| {
                        if next.visit == state_visit {
                            // already explored
                            self[next.node_map].rc += 1;
                            if let Some((i, ..)) = self.tmp_stack.last_mut() {
                                *i += 1;
                            } else {
                                break_dfs = true;
                            }
                        } else {
                            let p = self.a.insert(OpNode {
                                rc: 1,
                                nzbw: next.nzbw,
                                op: Op::Invalid,
                                err: None,
                                location: next.location,
                                visit: node_visit,
                            });
                            next.node_map = p;
                            next.visit = state_visit;
                            if record_added {
                                self.tmp_pnodes_added.push(p);
                            }
                            self.tmp_stack.push((0, p, p_next));
                        }
                    });
                }
                if break_dfs {
                    break
                }
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
        for assertion in &self.assertions {
            if !self.note_arena.contains(*assertion) {
                return Err(EvalError::OtherString(format!(
                    "assertion {assertion} not contained in note arena"
                )))
            }
        }
        for note in self.note_arena.vals() {
            if !self.a.contains(*note) {
                return Err(EvalError::OtherString(format!(
                    "note {note} not contained in node arena"
                )))
            }
        }
        for node in self.a.vals() {
            for operand in node.op.operands() {
                if !self.a.contains(*operand) {
                    return Err(EvalError::OtherString(format!(
                        "operand {operand} of node {node:?} not contained in node arena"
                    )))
                }
            }
        }
        // assert minimum rc counts
        let mut rc_counts = HashMap::<PNode, u64>::new();
        for node in self.a.vals() {
            for op in node.op.operands() {
                match rc_counts.entry(*op) {
                    Entry::Occupied(mut o) => {
                        *o.get_mut() = o.get().checked_add(1).unwrap();
                    }
                    Entry::Vacant(v) => {
                        v.insert(1);
                    }
                }
            }
        }
        for p_note in &self.assertions {
            match rc_counts.entry(self.note_arena[p_note]) {
                Entry::Occupied(mut o) => {
                    *o.get_mut() = o.get().checked_add(1).unwrap();
                }
                Entry::Vacant(v) => {
                    v.insert(1);
                }
            }
        }
        for (p_node, node) in &self.a {
            let expected = if let Some(x) = rc_counts.get(&p_node) {
                *x
            } else {
                continue
            };
            let rc = node.rc;
            if rc < expected {
                // TODO check that nothing else references the node
                return Err(EvalError::OtherString(format!(
                    "node {p_node} ({node:?}) has a {rc} reference count that is lower than the \
                     expected minimum of {expected}"
                )))
            }
        }
        Ok(())
    }

    /// Same as `assert_assertions` except it ignores opaques
    pub fn assert_assertions_weak(&mut self) -> Result<(), EvalError> {
        for (i, p_note) in self.assertions.iter().enumerate() {
            let p_node = self.note_arena[p_note];
            let node = &self[p_node];
            if node.nzbw.get() != 1 {
                return Err(EvalError::AssertionFailure(format!(
                    "assertion bit {i} ({p_note}) is not a single bit, assertion location: {:?}",
                    node.location
                )))
            }
            if let Op::Literal(ref lit) = node.op {
                if lit.is_zero() {
                    return Err(EvalError::AssertionFailure(format!(
                        "assertion bits not all true, failed on bit {i} ({p_note}), assertion \
                         location: {:?}",
                        self[p_node].location
                    )))
                }
            }
        }
        Ok(())
    }

    /// Checks that all assertion bits are literal `0x1u1`s.
    pub fn assert_assertions(&mut self) -> Result<(), EvalError> {
        for (i, p_note) in self.assertions.iter().enumerate() {
            let p_node = self.note_arena[p_note];
            let node = &self[p_node];
            if node.nzbw.get() != 1 {
                return Err(EvalError::AssertionFailure(format!(
                    "assertion bit {i} ({p_note}) is not a single bit, assertion location: {:?}",
                    node.location
                )))
            }
            if let Op::Literal(ref lit) = node.op {
                if lit.is_zero() {
                    return Err(EvalError::AssertionFailure(format!(
                        "assertion bits not all true, failed on bit {i} ({p_note}), assertion \
                         location: {:?}",
                        self[p_node].location
                    )))
                }
            } else {
                return Err(EvalError::AssertionFailure(format!(
                    "assertion bit {i} ({p_note}) is not a literal (it is {:?}), assertion \
                     location: {:?}",
                    node.op, node.location
                )))
            }
        }
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
            if lit.bw() == USIZE_BITS {
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

    /// Trims the tree starting at leaf `p`, assuming `p` has a zero reference
    /// count
    pub fn trim_zero_rc_tree(&mut self, p: PNode) -> Result<(), EvalError> {
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
        Ok(())
    }

    /// Decrements the reference count on `p`, and propogating removals if it
    /// goes to zero. Returns an error if the reference count was already zero
    /// (which means invariant breakage occured earlier).
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
    /// nodes not including `ptr` are colored with `visit`. For performance, an
    /// epoch should be created per graft.
    pub fn graft(
        &mut self,
        epoch: StateEpoch,
        ptr: PNode,
        node_visit: u64,
        output_and_operands: &[PState],
    ) -> Result<(), EvalError> {
        #[cfg(debug_assertions)]
        {
            if (self[ptr].op.operands_len() + 1) != output_and_operands.len() {
                return Err(EvalError::WrongNumberOfOperands)
            }
            for (i, op) in self[ptr].op.operands().iter().enumerate() {
                let current_nzbw = output_and_operands[i + 1].get_nzbw();
                let current_is_opaque = output_and_operands[i + 1].get_op().is_opaque();
                if self[op].nzbw != current_nzbw {
                    return Err(EvalError::OtherString(format!(
                        "operand {}: a bitwidth of {:?} is trying to be grafted to a bitwidth of \
                         {:?}",
                        i, current_nzbw, self[op].nzbw
                    )))
                }
                if !current_is_opaque {
                    return Err(EvalError::ExpectedOpaque)
                }
            }
            if self[ptr].nzbw != output_and_operands[0].get_nzbw() {
                return Err(EvalError::WrongBitwidth)
            }
        }
        self.tmp_pnodes_added.clear();
        let err = self.add_chain(
            &epoch,
            epoch.latest_state(),
            epoch.next_visit_gen(),
            node_visit,
            true,
        );
        // note: the reference count invariant is suspended until after the remove
        // unused trees part

        // self.render_to_svg_file(std::path::PathBuf::from("debug_graft.svg"))
        //    .unwrap();
        err.unwrap();

        // graft inputs
        for i in 1..output_and_operands.len() {
            let grafted = epoch.pstate_to_pnode(output_and_operands[i]);
            let grafted = if self.a.contains(grafted) {
                Some(grafted)
            } else {
                None
            };
            let graftee = self[ptr].op.operands()[i - 1];
            if let Some(grafted) = grafted {
                // change the grafted `Opaque` into a `Copy` that routes to the graftee instead
                // of needing to change all the operands of potentially many
                // internal nodes.
                self[grafted].op = Copy([graftee]);
                // do not increment rc, because it was referenced before
            } else {
                // else the operand is not used because it was optimized away, keep this because
                // this is removing a tree outside of the grafted part
                self.dec_rc(graftee).unwrap();
            }
        }

        // graft output
        let grafted = epoch.pstate_to_pnode(output_and_operands[0]);
        self[ptr].op = Copy([grafted]);
        self[grafted].rc = self[grafted].rc.checked_add(1).unwrap();
        drop(epoch);
        while let Some(p_node) = self.tmp_pnodes_added.pop() {
            // remove unused trees
            if let Some(node) = self.a.get(p_node) {
                if node.rc == 0 {
                    self.trim_zero_rc_tree(p_node).unwrap();
                }
            }
        }
        //self.verify_integrity().unwrap();
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

impl Default for OpDag {
    fn default() -> Self {
        Self::new()
    }
}
