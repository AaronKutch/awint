//! This DAG is for lowering into a LUT-only DAG

use std::{
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
    num::NonZeroUsize,
    ops::{Index, IndexMut},
    rc::Rc,
};

use triple_arena::{Arena, Ptr, PtrTrait};
use Op::*;

use super::node::P0;
use crate::{
    common::{Op, State},
    lowering::{EvalError, Node, PtrEqRc},
};

#[derive(Debug)]
pub struct Dag<P: PtrTrait> {
    pub dag: Arena<P, Node<P>>,
    pub roots: Vec<Ptr<P>>,
    pub leaves: Vec<Ptr<P>>,
    pub noted_roots: Vec<Ptr<P>>,
    pub noted_leaves: Vec<Ptr<P>>,
}

impl<P: PtrTrait, B: Borrow<Ptr<P>>> Index<B> for Dag<P> {
    type Output = Node<P>;

    fn index(&self, index: B) -> &Node<P> {
        self.dag.get(*index.borrow()).unwrap()
    }
}

impl<P: PtrTrait, B: Borrow<Ptr<P>>> IndexMut<B> for Dag<P> {
    fn index_mut(&mut self, index: B) -> &mut Node<P> {
        self.dag.get_mut(*index.borrow()).unwrap()
    }
}

impl<P: PtrTrait> Dag<P> {
    /// Constructs a directed acyclic graph from the leaf sinks of a mimicking
    /// version. The optional `roots` make the `Dag` keep track of select nodes
    /// in `noted_roots`, which must be `Opaque` nodes.
    pub fn new(leaves: Vec<Rc<State>>, roots: Vec<Rc<State>>) -> (Self, Result<(), EvalError>) {
        // keeps track if a mimick node is already tracked in the arena
        let mut lowerings: HashMap<PtrEqRc, Ptr<P>> = HashMap::new();
        // used later for when all nodes are allocated
        let mut raisings: Vec<(Ptr<P>, PtrEqRc)> = Vec::new();
        // keep a frontier which will guarantee that the whole mimick DAG is explored,
        // and keep track of dependents
        let mut frontier = leaves.clone();
        let mut dag: Arena<P, Node<P>> = Arena::new();
        // because some nodes may not be in the arena yet, we have to bootstrap
        // dependencies by looking up the source later (source, sink)
        let mut deps: Vec<(PtrEqRc, Ptr<P>)> = Vec::new();
        while let Some(next) = frontier.pop() {
            match lowerings.entry(PtrEqRc(Rc::clone(&next))) {
                Entry::Occupied(_) => (),
                Entry::Vacant(v) => {
                    let sink = dag.insert(Node {
                        nzbw: next.nzbw,
                        op: next.op.clone(),
                        ops: Vec::new(),
                        deps: Vec::new(),
                        err: None,
                    });
                    v.insert(sink);
                    for source in next.ops.clone() {
                        deps.push((PtrEqRc(Rc::clone(&source)), sink));
                        frontier.push(source);
                    }
                    raisings.push((sink, PtrEqRc(Rc::clone(&next))));
                }
            }
        }
        // set all dependents
        for dep in &deps {
            dag[lowerings[&dep.0]].deps.push(dep.1);
        }
        // set all operands
        for (ptr, rc) in raisings {
            for i in 0..rc.0.ops.len() {
                dag[ptr].ops.push(lowerings[&PtrEqRc(rc.0.ops[i].clone())]);
            }
        }
        // We face a problem where we want to keep active references to the input
        // `leaves` even if one leaf is in the computation tree of another (and
        // simplifications could destroy the value there). We face another problem in
        // the algorithms where they would have to consider both `deps` and
        // `Dag.leaves` for liveness and rerouting. To solve both these, we add an extra
        // `Opaque` layer, and then only `Opaque` handling needs to consider external
        // liveness. The `Opaque` layer needs to be added unconditionally or else
        // algorithms may not be able to determine what is added `Opaque` nodes and what
        // isn't.
        let mut noted_leaves = vec![];
        for leaf in leaves {
            let ptr = lowerings[&PtrEqRc(leaf)];
            let node = dag.insert(Node {
                nzbw: dag[ptr].nzbw,
                op: Op::Opaque,
                ops: vec![ptr],
                deps: vec![],
                err: None,
            });
            dag[ptr].deps.push(node);
            noted_leaves.push(node);
        }
        // handle noted roots
        let mut err = Ok(());
        let mut noted_roots = vec![];
        for (i, root) in roots.into_iter().enumerate() {
            match lowerings.entry(PtrEqRc(root)) {
                Entry::Occupied(o) => {
                    if !matches!(dag[o.get()].op, Opaque) {
                        dag[o.get()].err = Some(EvalError::ExpectedOpaque);
                        err = Err(EvalError::ExpectedOpaque);
                    }
                    noted_roots.push(*o.get());
                }
                Entry::Vacant(_) => {
                    err = Err(EvalError::OtherString(format!(
                        "root {} is not included in DAG reached by the leaves",
                        i
                    )));
                }
            }
        }
        let mut roots = vec![];
        for p in dag.ptrs() {
            if dag[p].ops.is_empty() {
                roots.push(p);
            }
        }
        let mut leaves = vec![];
        for p in dag.ptrs() {
            if dag[p].deps.is_empty() {
                leaves.push(p);
            }
        }
        (
            Self {
                dag,
                roots,
                leaves,
                noted_leaves,
                noted_roots,
            },
            err,
        )
    }

    /// Checks that the DAG is not broken and that the bitwidth checks work.
    /// Note that if the DAG is evaluated, there may be invalid operand value
    /// errors.
    pub fn verify_integrity(&mut self) -> Result<(), EvalError> {
        'outer: for p in self.ptrs() {
            let len = self[p].ops.len();
            if let Some(expected) = self[p].op.operands_len() {
                if expected != len {
                    self[p].err = Some(EvalError::WrongNumberOfOperands);
                }
            }
            let mut bitwidths = vec![];
            for i in 0..self[p].ops.len() {
                let op = self[p].ops[i];
                if let Some(nzbw) = self[op].nzbw {
                    bitwidths.push(nzbw.get());
                } else {
                    // can't do anything
                    continue 'outer
                }
                if !self[op].deps.contains(&p) {
                    self[p].err = Some(EvalError::InvalidPtr);
                }
            }
            if let Some(nzbw) = self[p].nzbw {
                if self[p].op.check_bitwidths(nzbw.get(), &bitwidths) {
                    self[p].err = Some(EvalError::WrongBitwidth);
                }
            }
        }
        for p in self.roots() {
            if self.dag.get(*p).is_none() {
                return Err(EvalError::InvalidPtr)
            }
        }
        for p in self.leaves() {
            if self.dag.get(*p).is_none() {
                return Err(EvalError::InvalidPtr)
            }
        }
        for p in &self.noted_roots {
            if self.dag.get(*p).is_none() {
                return Err(EvalError::InvalidPtr)
            }
        }
        for p in &self.noted_leaves {
            if self.dag.get(*p).is_none() {
                return Err(EvalError::InvalidPtr)
            }
        }
        for v in self.dag.vals() {
            if let Some(ref err) = v.err {
                return Err(err.clone())
            }
        }
        Ok(())
    }

    /// Returns a list of pointers to all nodes in no particular order
    pub fn ptrs(&self) -> Vec<Ptr<P>> {
        self.dag.ptrs().collect()
    }

    /// Returns all source roots that have no operands
    pub fn roots(&self) -> &[Ptr<P>] {
        &self.roots
    }

    /// Returns all sink leaves that have no dependents
    pub fn leaves(&self) -> &[Ptr<P>] {
        &self.leaves
    }

    pub fn get_bw<B: Borrow<Ptr<P>>>(&self, ptr: B) -> Result<NonZeroUsize, EvalError> {
        if let Some(w) = self[ptr].nzbw {
            Ok(w)
        } else {
            Err(EvalError::NonStaticBitwidth)
        }
    }

    pub fn get_2ops<B: Borrow<Ptr<P>>>(&self, ptr: B) -> Result<[Ptr<P>; 2], EvalError> {
        if let [a, b] = self[ptr].ops[..] {
            Ok([a, b])
        } else {
            Err(EvalError::WrongNumberOfOperands)
        }
    }

    pub fn strip_opaque_leaf<B: Borrow<Ptr<P>>>(&mut self, ptr: B) -> Result<Ptr<P>, EvalError> {
        let ptr = *ptr.borrow();
        if matches!(&self[ptr].op, Opaque) && (self[ptr].ops.len() == 1) {
            let res = Ok(self[ptr].ops[0]);
            self.dag.remove(ptr).unwrap();
            res
        } else {
            Err(EvalError::OtherString(format!(
                "is not opaque leaf: {:?}",
                self[ptr]
            )))
        }
    }

    /// Forbidden meta pseudo-DSL techniques in which the node at `ptr` is
    /// replaced by a set of lowered `Rc<State>` nodes with interfaces `ops` and
    /// `deps` corresponding to the original interfaces.
    pub fn graft(
        &mut self,
        ptr: Ptr<P>,
        leaf: Rc<State>,
        roots: Vec<Rc<State>>,
    ) -> Result<(), EvalError> {
        let (mut dag, err) = Dag::<P0>::new(vec![leaf], roots);
        //dag.render_to_svg_file(std::path::PathBuf::from("debug.svg")).unwrap();
        err?;
        dag.verify_integrity()?;
        dag.eval()?;
        self.graft_dag(ptr, dag)
    }

    /// Uses the noted leaf and noted roots of `dag` to replace the interfaces
    /// of the nodes using `ptr`
    pub fn graft_dag<Q: PtrTrait>(
        &mut self,
        ptr: Ptr<P>,
        mut dag: Dag<Q>,
    ) -> Result<(), EvalError> {
        if dag.noted_leaves.len() != 1 {
            return Err(EvalError::OtherStr("the number of noted_leaves is not 1"))
        }
        let noted_leaf = dag.noted_leaves[0];
        if dag.noted_roots.len() != self[ptr].ops.len() {
            return Err(EvalError::OtherStr(
                "the number of noted_leaves does not equal the number of operands",
            ))
        }
        // first lower without ops or deps and get a backwards translation
        let mut translate = Arena::<Q, Ptr<P>>::new();
        translate.clone_from_with(&dag.dag, |_, _| Ptr::invalid());
        for (q, node) in &mut dag.dag {
            translate[q] = self.dag.insert(Node {
                nzbw: node.nzbw,
                op: node.op.take(),
                deps: vec![],
                ops: vec![],
                err: node.err.take(),
            });
        }
        // connect internals
        for (q, mut node) in dag.dag.drain() {
            self[translate[q]].ops = node.ops.drain(..).map(|q| translate[q]).collect();
            self[translate[q]].deps = node.deps.drain(..).map(|q| translate[q]).collect();
        }
        // connect interfaces, remove opaques
        for i in 0..dag.noted_roots.len() {
            let root = translate[dag.noted_roots[i]];
            if !(matches!(&self[root].op, Opaque) && self[root].ops.is_empty()) {
                return Err(EvalError::OtherString(format!(
                    "is not opaque root: {:?}",
                    self[root]
                )))
            }
            // change to a `Copy` or else we have to do a lot of linear lookups
            self[root].op = Copy;
            let graft_point = self[ptr].ops[i];
            let dep_i = self[graft_point]
                .deps
                .iter()
                .position(|p| *p == ptr)
                .unwrap();
            self[graft_point].deps[dep_i] = root;
        }
        let leaf = translate[noted_leaf];
        let new_op = self.strip_opaque_leaf(leaf)?;
        for i in 0..self[ptr].deps.len() {
            let graft_point = self[ptr].deps[i];
            let op_i = self[graft_point]
                .ops
                .iter()
                .position(|p| *p == ptr)
                .unwrap();
            self[graft_point].ops[op_i] = new_op;
        }
        // remove old node
        self.dag.remove(ptr).unwrap();
        Ok(())
    }

    /// Always renders to file, and then returns errors
    #[cfg(feature = "debug")]
    pub fn render_to_svg_file(&mut self, out_file: std::path::PathBuf) -> Result<(), EvalError> {
        let res = self.verify_integrity();
        triple_arena_render::render_to_svg_file(&self.dag, false, out_file).unwrap();
        res
    }

    /// Always renders to file, and then returns errors
    #[cfg(feature = "debug")]
    pub fn eval_and_render_to_svg_file(
        &mut self,
        out_file: std::path::PathBuf,
    ) -> Result<(), EvalError> {
        let res0 = self.verify_integrity();
        if res0.is_err() {
            triple_arena_render::render_to_svg_file(&self.dag, false, out_file).unwrap();
            res0
        } else {
            let res1 = self.eval();
            triple_arena_render::render_to_svg_file(&self.dag, false, out_file).unwrap();
            res1
        }
    }
}
