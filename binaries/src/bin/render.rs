use std::{
    collections::{HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use awint_dag::{arena::Ptr, lowering::Dag, mimick::Lineage};
use common::dag_input::dag_input;

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
fn main() {
    let out_file = PathBuf::from("./rendered.svg".to_owned());
    drop(fs::remove_file(&out_file));

    let leaves = dag_input();
    let dag = Dag::new(leaves);

    // DFS for topological sort
    let mut sorted: Vec<Ptr> = Vec::new();
    let mut done_nodes: HashSet<Ptr> = HashSet::new();
    let mut node: Vec<Ptr> = Vec::new();
    let mut dep_i: Vec<usize> = Vec::new();
    let roots: Vec<Ptr> = dag.roots();
    for root in &roots {
        node.push(*root);
        dep_i.push(0);
        loop {
            let current = node[node.len() - 1];
            match dag.dag[current].deps.get(dep_i[dep_i.len() - 1]) {
                Some(dependent) => {
                    if done_nodes.contains(&dependent) {
                        let len = dep_i.len();
                        dep_i[len - 1] += 1;
                    } else {
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
                    let len = dep_i.len();
                    dep_i[len - 1] += 1;
                }
            }
        }
    }

    // we structure the graph by looking at the first operand and seeing how far
    // back that operand is used
    let mut first_operands: Vec<Ptr> = Vec::new();
    let mut done_nodes: HashSet<Ptr> = HashSet::new();
    let mut node: Vec<Ptr> = Vec::new();
    let mut op_i: Vec<usize> = Vec::new();
    let leaves = dag.leaves();
    for leaf in &leaves {
        //first_operands.push(*leaf);
        node.push(*leaf);
        op_i.push(0);
        loop {
            let current = node[node.len() - 1];
            let i = op_i[op_i.len() - 1];
            match dag.dag[current].ops.get(i) {
                Some(operand) => {
                    if done_nodes.contains(operand) {
                        let len = op_i.len();
                        op_i[len - 1] += 1;
                    } else {
                        if i != 0 {
                            dbg!();
                            first_operands.push(current);
                        }
                        node.push(*operand);
                        op_i.push(0);
                    }
                }
                None => {
                    // no more dependents, backtrack
                    done_nodes.insert(current);
                    node.pop();
                    op_i.pop();
                    if node.is_empty() {
                        break
                    }
                    let len = op_i.len();
                    op_i[len - 1] += 1;
                }
            }
        }
    }

    //dbg!(&dag);
    //dbg!(dag.roots(), dag.leaves());
    dbg!(&sorted, &first_operands);

    let mut grid: Vec<Vec<Ptr>> = Vec::new();
    for op in first_operands {
        let mut current = op;
        let mut vertical = Vec::new();
        loop {
            vertical.push(current);
            match dag.dag[current].ops.first() {
                Some(op) => current = *op,
                None => break,
            }
        }
        grid.push(vertical);
    }

    let mut s = String::new();
    //s += "<g fill="" />";

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
