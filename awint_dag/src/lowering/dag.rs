//! This DAG is for lowering into a LUT-only DAG

use std::{
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
    num::NonZeroUsize,
    ops::{Index, IndexMut},
    rc::Rc,
};

use triple_arena::{Arena, Ptr, PtrTrait};

use crate::{
    common::{Op, State},
    lowering::{EvalError, Node, PtrEqRc},
};

#[derive(Debug)]
pub struct Dag<P: PtrTrait> {
    pub dag: Arena<P, Node<P>>,
    pub leaves: Vec<Ptr<P>>,
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
    /// version
    pub fn new(leaves: Vec<Rc<State>>) -> Self {
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
        // liveness.
        let mut real_leaves = vec![];
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
            real_leaves.push(node);
        }
        Self {
            dag,
            leaves: real_leaves,
        }
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
        for p in self.leaves() {
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
    pub fn roots(&self) -> Vec<Ptr<P>> {
        let mut v = Vec::new();
        for p in self.ptrs() {
            if self[p].ops.is_empty() {
                v.push(p);
            }
        }
        v
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
