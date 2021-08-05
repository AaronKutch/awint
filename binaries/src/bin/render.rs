use std::{
    collections::{hash_map::Entry::*, HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use awint_dag::{arena::Ptr, lowering::Dag};
use common::dag_input::dag_input;

fn main() {
    let out_file = PathBuf::from("./rendered.svg".to_owned());
    drop(fs::remove_file(&out_file));

    let leaves = dag_input();
    let dag = Dag::new(leaves);

    // DFS for topological sort
    let mut sorted: Vec<Ptr> = vec![];
    // done nodes
    let mut done_nodes: HashSet<Ptr> = HashSet::new();
    // frontier
    let mut node: Vec<Ptr> = vec![];
    // path through dependencies
    let mut dep_i: Vec<usize> = vec![];
    let roots: Vec<Ptr> = dag.roots();
    for root in &roots {
        node.push(*root);
        dep_i.push(0);
        loop {
            let current = node[node.len() - 1];
            match dag.dag[current].deps.get(dep_i[dep_i.len() - 1]) {
                Some(dependent) => {
                    if done_nodes.contains(dependent) {
                        // if node was already explored, check the next dependency
                        let len = dep_i.len();
                        dep_i[len - 1] += 1;
                    } else {
                        // else explore further
                        node.push(*dependent);
                        dep_i.push(0);
                    }
                }
                None => {
                    // no more dependents, backtrack
                    sorted.push(current);
                    done_nodes.insert(current);
                    node.pop();
                    dep_i.pop();
                    if node.is_empty() {
                        break
                    }
                    // check next dependency
                    let len = dep_i.len();
                    dep_i[len - 1] += 1;
                }
            }
        }
    }

    // map `Ptr`s to their position in `sorted`
    let mut sort_map: HashMap<Ptr, usize> = HashMap::new();
    for (i, ptr) in sorted.iter().enumerate() {
        sort_map.insert(*ptr, i);
    }

    // we structure the graph by looking at the first operand of an operation and
    // constructing a "lineage" where the same backing storage is being used as the
    // first operand. We use a special "lineage search" that constructs a vector of
    // lineages. It works by selecting any unexplored node, then adding on to the
    // lineage all the way until a root is reached. If the leafward parts of the
    // lineage are not touched on the first exploration, later explorations through
    // the same lineage will overwrite the lineage number.
    let mut n = 0;
    let mut lineage_map: HashMap<Ptr, usize> = HashMap::new();
    let mut lineage_leaves: HashMap<usize, Ptr> = HashMap::new();
    for ptr in dag.list_ptrs() {
        if !lineage_map.contains_key(&ptr) {
            let mut next = ptr;
            lineage_map.insert(next, n);
            lineage_leaves.insert(n, next);
            while let Some(next_zeroeth) = dag.dag[next].ops.get(0) {
                next = *next_zeroeth;
                match lineage_map.entry(next) {
                    Occupied(mut o) => {
                        // remove prior firsts
                        let _ = lineage_leaves.remove(o.get());
                        o.insert(n);
                    }
                    Vacant(v) => {
                        v.insert(n);
                    }
                }
            }
            n += 1;
        }
    }

    // get ordered lineages
    let mut lineages: Vec<Vec<Ptr>> = vec![];
    for leaf in lineage_leaves {
        let mut next = leaf.1;
        let mut lineage = vec![next];
        while let Some(next_zeroeth) = dag.dag[next].ops.get(0) {
            next = *next_zeroeth;
            lineage.push(next);
        }
        lineages.push(lineage);
    }
    // sort by the the topological sorting of the leaves
    lineages.sort_by(|a, b| sort_map[&a[0]].cmp(&sort_map[&b[0]]));

    // make a new map of ptrs to their lineage and lineage positions
    let mut lineage_map: HashMap<Ptr, (usize, usize)> = HashMap::new();
    for (lineage_i, lineage) in lineages.iter().enumerate() {
        for (ptr_i, ptr) in lineage.iter().enumerate() {
            lineage_map.insert(*ptr, (lineage_i, ptr_i));
        }
    }

    // Finally, make a grid such that any dependency must flow one way. The second
    // element in the tuple says how far back from the leaf line the node should be
    // placed.
    let mut grid: Vec<Vec<(Ptr, usize)>> = vec![];
    for lineage in &lineages {
        let mut vertical = vec![];
        for ptr in &*lineage {
            vertical.push((*ptr, sort_map[ptr]));
        }
        grid.push(vertical);
    }
    // compress as much as possible
    let mut changed;
    loop {
        changed = false;
        for vertical in &mut grid {
            for slot in &mut *vertical {
                let mut pos = 0;
                for dep in &dag.dag[slot.0].deps {
                    pos = std::cmp::max(pos, sort_map[dep] + 1);
                }
                if pos < slot.1 {
                    // There is room to slide down
                    (*slot).1 = pos;
                    sort_map.insert(slot.0, pos).unwrap();
                    changed = true;
                }
            }
        }
        if !changed {
            break
        }
    }

    // create the SVG code

    // example code for future reference
    //<circle cx="128" cy="128" r="128" fill="orange"/>
    //<rect fill="#f00" x="64" y="64" width="64" height="64"/>
    //<text x="64" y="64" font-size="16" font-family="monospace"
    //<text font-weight="normal">oO0iIlL</text>
    //<!-- L is for line Q is for quadratic C is for cubic -->>
    //<path d="M 0,0 C 100,0 100,100 0,100" stroke="#00f" fill="#000a"/>
    //<!-- grouping element <g> </g> that can apply common modifiers such as
    //<!-- `opacity="0.5"` -->
    //<!--
    //<rect fill="#fff" stroke="#000" x="0" y="0" width="300" height="300"/>
    //<rect x="25" y="25" width="500" height="200" fill="none" stroke-width="4"
    //<rect stroke="pink" />
    //<circle cx="125" cy="125" r="75" fill="orange" />
    //<polyline points="50,150 50,200 200,200 200,100" stroke="red" stroke-width="4"
    //<polyline fill="none" />
    //<line x1="50" y1="50" x2="200" y2="200" stroke="blue" stroke-width="4" />
    //-->
    let mut s = String::new();
    //s += "<g fill="" />";

    // #111 background
    // gray #877
    // pink #f08
    // orange #e30
    // yellow #970
    // green #090
    // cyan #0a7
    // blue #08f
    //s += "<rect fill=\"#111\">\n";

    let viewbox = (256, 256);
    let output = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n\
        <!DOCTYPE svg PUBLIC \"-//W3C//DTD SVG 1.1//EN\" \
        \"http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd\">\n\
        <svg preserveAspectRatio=\"meet\" viewBox=\"0 0 {} {}\" width=\"100%\" height=\"100%\" \
        version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\">\n\
        {}\n\
        </svg>",
        viewbox.0,
        viewbox.1,
        s,
    );
    OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(out_file)
        .unwrap()
        .write_all(output.as_bytes())
        .unwrap();
}
