//! This DAG is for lowering into a LUT-only DAG

use std::{
    collections::{hash_map::Entry, HashMap},
    rc::Rc,
};

use crate::{
    lowering::{Arena, Dag, Node, Op, Ptr, PtrEqRc},
    mimick,
};

impl Dag {
    /// Constructs a directed acyclic graph from the root sinks of a mimicking
    /// version
    pub fn new(roots: Vec<Rc<mimick::Op>>) -> Self {
        // keeps track if a mimick node is already tracked in the arena
        let mut mp_to_p: HashMap<PtrEqRc, Ptr> = HashMap::new();
        // keep a frontier which will guarantee that the whole mimick DAG is explored,
        // and keep track of dependents
        let mut frontier: Vec<Rc<mimick::Op>> = roots;
        let mut dag: Arena<Node> = Arena::new();
        // because some nodes may not be in the arena yet, we have to bootstrap
        // dependencies by looking up the source later (source, sink)
        let mut deps: Vec<(PtrEqRc, Ptr)> = Vec::new();
        // all leaf sources
        let mut leaves: Vec<Ptr> = Vec::new();
        while let Some(next) = frontier.pop() {
            match mp_to_p.entry(PtrEqRc(Rc::clone(&next))) {
                Entry::Occupied(_) => (),
                Entry::Vacant(v) => {
                    let sink = dag.insert(Node::new(Op::Unlowered(next.clone())));
                    v.insert(sink);
                    let sources = next.list_sources();
                    if sources.is_empty() {
                        leaves.push(sink);
                    } else {
                        for source in sources {
                            deps.push((PtrEqRc(Rc::clone(&source)), sink));
                            frontier.push(source);
                        }
                    }
                }
            }
        }
        // set all dependents
        for dep in &deps {
            dag[mp_to_p[&dep.0]].deps.push(dep.1);
            dag[dep.1].num_unlowered_sources += 1;
        }
        // lower from `Op::Unlowered` starting from leaves
        let mut frontier = leaves;
        //dbg!(frontier);
        while let Some(p) = frontier.pop() {
            let node = &mut dag[p];
            let lowered = node.op.lower(&mp_to_p).unwrap();
            node.op = lowered;
            for dep in node.deps.clone() {
                let dependent = &mut dag[dep];
                dependent.num_unlowered_sources -= 1;
                if dependent.num_unlowered_sources == 0 {
                    frontier.push(dep);
                }
            }
        }
        Self { dag }
    }
}
