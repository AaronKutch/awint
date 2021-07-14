//! This DAG is for lowering into a LUT-only DAG

use std::{
    collections::{hash_map::Entry, HashMap},
    num::NonZeroUsize,
    rc::Rc,
};

use crate::{
    lowering::{Arena, Op, Ptr},
    mimick,
};

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
struct Node {
    bw: Option<NonZeroUsize>,
    op: Op,
    /// Lists nodes that use this one as a source
    dependents: Vec<Ptr>,
}

impl Node {
    pub fn new(op: Op) -> Self {
        Node {
            bw: None,
            op,
            dependents: Vec::new(),
        }
    }
}

pub struct Dag {
    dag: Arena<Node>,
}

/// Defines equality using Rc::ptr_eq
#[allow(clippy::derive_hash_xor_eq)] // If `ptr_eq` is true, the `Hash` defined on `Rc` also agrees
#[derive(Debug, Hash, Clone, Eq)]
struct PtrEqRc(Rc<mimick::Op>);

impl PartialEq for PtrEqRc {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Dag {
    /// Constructs a directed acyclic graph from the root sinks of a mimicking
    /// version
    pub fn new(root_sinks: Vec<Rc<mimick::Op>>) -> Self {
        // keeps track if a mimick node is already tracked in the arena
        let mut rc_nodes: HashMap<PtrEqRc, Ptr> = HashMap::new();
        // keep a frontier which will guarantee that the whole mimick DAG is explored,
        // and keep track of dependents
        let mut frontier: Vec<Rc<mimick::Op>> = root_sinks;
        let mut dag: Arena<Node> = Arena::new();
        // because some nodes may not be in the arena yet, we have to bootstrap
        // dependencies by looking up the source later (source, sink)
        let mut deps: Vec<(PtrEqRc, Ptr)> = Vec::new();
        while let Some(next) = frontier.pop() {
            match rc_nodes.entry(PtrEqRc(Rc::clone(&next))) {
                Entry::Occupied(_) => (),
                Entry::Vacant(v) => {
                    let sink = dag.insert(Node::new(Op::Unlowered(next.clone())));
                    v.insert(sink);
                    for source in next.list_sources() {
                        deps.push((PtrEqRc(Rc::clone(&source)), sink));
                        frontier.push(source);
                    }
                }
            }
        }
        // set all dependents
        for dep in deps {
            let source = rc_nodes[&dep.0];
            dag.get_mut(source).unwrap().dependents.push(dep.1);
        }
        Self { dag }
    }

    pub fn lower(&mut self) {
        let _ = self.dag.is_empty();
    }
}
