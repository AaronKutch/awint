use std::{
    cmp::max,
    collections::{hash_map::Entry::*, HashMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
};

use awint_dag::{
    arena::Ptr,
    lowering::{Dag, Node},
};
use common::dag_input::dag_input;

// for calibration
//<?xml version="1.0" encoding="UTF-8" standalone="no"?>
//<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN"
//"http://www.w3.org/Graphics///SVG/1.1/DTD/svg11.dtd">
//<svg preserveAspectRatio="meet" viewBox="0 0 400 200" width="100%"
//height="100%" version="1.1" xmlns="http://www.w3.org/2000/svg">
//<rect fill="#111" x="0" y="64" width="200" height="16"/>
//<text fill="red" font-size="16" font-family="monospace" x="0"
//<text //y="75">0123456789abcdef=|_</text>
//<line x1="0" y1="80" x2="154" y2="82" stroke="#f08" stroke-width="2" />
//</svg>

const FONT_FAMILY: &str = "monospace";
const FONT_SIZE: i32 = 16;
const FONT_WX: i32 = 10;
const FONT_WY: i32 = 16;
// Fonts tend to hang downwards below the point at which they are written, so
// this corrects that
const FONT_ADJUST_Y: i32 = -5;
const PAD: i32 = 4;
const NODE_PAD: i32 = 32;
const RECT_OUTLINE_WIDTH: i32 = 1;

#[derive(Debug, Clone, Copy)]
enum TextType {
    Operand,
    Operation,
}

struct RenderNode {
    pub rects: Vec<(i32, i32, i32, i32)>,
    pub text: Vec<((i32, i32), TextType, &'static str)>,
    pub input_points: Vec<(i32, i32)>,
    pub output_point: (i32, i32),
    pub wx: i32,
    pub wy: i32,
}

impl RenderNode {
    pub fn new(node: &Node) -> Self {
        let operand_names = node.op.operand_names();
        let operation_name = node.op.operation_name();
        let total_operand_len: i32 = operand_names.iter().map(|name| name.len() as i32).sum();
        let operation_len = operation_name.len() as i32;
        let min_operands_wx =
            FONT_WX * total_operand_len + (2 * PAD) * (operand_names.len() as i32);
        let min_operation_wx = FONT_WX * operation_len + 2 * PAD;
        let wx = max(min_operands_wx, min_operation_wx);
        // for spreading out inputs
        let extra_space = if min_operands_wx < min_operation_wx {
            min_operation_wx - min_operands_wx
        } else {
            0
        };
        let mut rects = vec![];
        let mut text = vec![];
        let mut input_points = vec![];
        let mut wy = 0;
        let textbox_wy = 2 * PAD + FONT_WY;
        if operand_names.len() > 1 {
            let individual_spaces = extra_space / (operand_names.len() as i32 - 1);
            let mut x_progression = 0;
            for name in &operand_names {
                let rect = (
                    x_progression,
                    0,
                    FONT_WX * (name.len() as i32) + 2 * PAD,
                    textbox_wy,
                );
                text.push((
                    (rect.0 + PAD, (rect.1 + rect.3) - PAD),
                    TextType::Operand,
                    *name,
                ));
                let center_x = rect.0 + (rect.2 / 2);
                input_points.push((center_x, 0));
                x_progression += rect.2 + individual_spaces;
                rects.push(rect);
            }
            wy += textbox_wy;
        }
        let rect = (0, wy, wx, textbox_wy);
        wy += rect.3;
        let center_x = rect.0 + (rect.2 / 2);
        text.push((
            (
                center_x - ((FONT_WX * (operation_name.len() as i32)) / 2),
                wy - PAD,
            ),
            TextType::Operation,
            operation_name,
        ));
        rects.push(rect);

        Self {
            rects,
            text,
            input_points,
            output_point: (center_x, wy),
            wx,
            wy,
        }
    }

    pub fn translate(&mut self, mov: (i32, i32)) {
        for rect in &mut self.rects {
            rect.0 += mov.0;
            rect.1 += mov.1;
        }
        for tmp in &mut self.text {
            tmp.0 .0 += mov.0;
            tmp.0 .1 += mov.1;
        }
        for point in &mut self.input_points {
            point.0 += mov.0;
            point.1 += mov.1;
        }
        self.output_point.0 += mov.0;
        self.output_point.1 += mov.1;
    }
}

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

    let mut max_y_nodes = 0;
    for vert in &grid {
        for slot in &*vert {
            max_y_nodes = max(max_y_nodes, slot.1);
        }
    }
    let mut grid_max_wx = vec![0; grid.len()];
    let mut grid_max_wy = vec![0; max_y_nodes + 1];
    let mut render_grid: Vec<Vec<Option<RenderNode>>> = vec![];

    for (x_i, vertical) in grid.iter().enumerate() {
        let mut tmp = vec![];
        for (ptr, pos) in &*vertical {
            for _ in tmp.len()..*pos {
                tmp.push(None);
            }
            let node = RenderNode::new(&dag.dag[ptr]);
            grid_max_wx[x_i] = max(grid_max_wx[x_i], node.wx);
            grid_max_wy[*pos] = max(grid_max_wy[*pos], node.wy);
            tmp.push(Some(node));
        }
        for _ in tmp.len()..max_y_nodes {
            tmp.push(None);
        }
        render_grid.push(tmp);
    }
    // `render_grid` is now completely rectangular and the `_max_` values are
    // correct

    // translate to final places
    let mut cumulative_x = vec![];
    let mut tot_wx = 0;
    for wx in &grid_max_wx {
        cumulative_x.push(tot_wx);
        tot_wx += wx + NODE_PAD;
    }
    let mut cumulative_y = vec![];
    let mut tot_wy = 0;
    for wy in &grid_max_wy {
        tot_wy += wy + NODE_PAD;
        cumulative_y.push(tot_wy);
    }
    for (i, vert) in render_grid.iter_mut().enumerate() {
        for (j, node) in vert.iter_mut().flatten().enumerate() {
            node.translate((cumulative_x[i], cumulative_y[j]));
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

    // #111 background
    // gray #877
    // pink #f08
    // orange #e30
    // yellow #970
    // green #090
    // cyan #0a7
    // blue #08f
    let mut s = String::new();
    //s += &format!("<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"#f0f\"/>", tmp.0
    // .0, tmp.0 .1);

    // rectangles and outlines first
    for vert in &render_grid {
        for node in (*vert).iter().flatten() {
            for rect in &node.rects {
                s += &format!(
                    "<rect fill=\"#111\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>\n",
                    rect.0, rect.1, rect.2, rect.3
                );
                // outline the rectangle
                s += &format!(
                    "<polyline stroke=\"#877\" stroke-width=\"{}\" points=\"{},{} {},{} {},{} \
                     {},{} {},{}\"  fill=\"#0000\"/>",
                    RECT_OUTLINE_WIDTH,
                    rect.0,
                    rect.1,
                    rect.0 + rect.2,
                    rect.1,
                    rect.0 + rect.2,
                    rect.1 + rect.3,
                    rect.0,
                    rect.1 + rect.3,
                    rect.0,
                    rect.1,
                );
            }
        }
    }

    // lines second

    // text last
    for vert in &render_grid {
        for node in (*vert).iter().flatten() {
            for tmp in &node.text {
                s += &format!(
                    "<text fill=\"#877\" font-size=\"{}\" font-family=\"{}\" x=\"{}\" \
                     y=\"{}\">{}</text>\n",
                    FONT_SIZE,
                    FONT_FAMILY,
                    tmp.0 .0,
                    tmp.0 .1 + FONT_ADJUST_Y,
                    tmp.2
                );
            }
        }
    }

    let viewbox = (tot_wx, tot_wy);
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
