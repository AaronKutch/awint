#![allow(clippy::many_single_char_names)]

use std::{ascii::AsciiExt, cmp::max, collections::{hash_map::Entry::*, HashMap, HashSet}, fs::{self, OpenOptions}, io::Write, path::PathBuf};

use awint::ExtAwi;
use awint_dag::{Op, arena::Ptr, lowering::{Dag, Node}};
use common::dag_input::dag_input;

// for calibration
//<?xml version="1.0" encoding="UTF-8" standalone="no"?>
//<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN"
//"http://www.w3.org/Graphics///SVG/1.1/DTD/svg11.dtd">
//<svg preserveAspectRatio="meet" viewBox="0 0 400 200" width="100%"
//height="100%" version="1.1" xmlns="http://www.w3.org/2000/svg">
//<rect fill="#111" x="0" y="64" width="200" height="16"/>
//<text fill="red" font-size="16" font-family="monospace" x="0" y="75">
//0123456789abcdef=|_
//</text>
//<line x1="0" y1="80" x2="154" y2="82" stroke="#f08" stroke-width="2" />
//</svg>

const FONT_FAMILY: &str = "monospace";
const FONT_SIZE: i32 = 16;
const FONT_WX: i32 = 10;
const FONT_WY: i32 = 16;
const INPUT_FONT_SIZE: i32 = 8;
const INPUT_FONT_WX: i32 = 5;
const INPUT_FONT_WY: i32 = 8;
const INPUT_PAD: i32 = 2;
// Fonts tend to hang downwards below the point at which they are written, so
// this corrects that
const FONT_ADJUST_Y: i32 = -5;
const PAD: i32 = 4;
const NODE_PAD: i32 = 32;
const RECT_OUTLINE_WIDTH: i32 = 1;
// This is calculated by using CIELAB colors, taking 4 combinations of 2 of max
// green, red, blue, and yellow at 50% lightness and 4 single max values at 75%
// lightness. The ff0000 and 009800 values together are not colorblind friendly,
// so I replace the red one with ff8080. There are two blue values 00a9ff and
// 00caff that are too close to each other, so I replace one with the grayscale
// b9b9b9. This has an interesting symmetry of 4 elements of magenta to yellow,
// 2 elements of green to cyan, 1 element of blue, and 1 gray element to close
// off the power of 2.
const COLORS: [&str; 8] = [
    "b9b9b9", "e0b500", "00a9ff", "ff8080", "00e6b6", "ff00be", "009800", "b900ff",
];

#[derive(Debug, Clone)]
struct RenderNode {
    pub ptr: Ptr,
    pub rects: Vec<(i32, i32, i32, i32)>,
    pub text: Vec<((i32, i32), i32, String)>,
    pub input_points: Vec<((i32, i32), Ptr)>,
    pub output_point: (i32, i32),
    pub wx: i32,
    pub wy: i32,
}

impl RenderNode {
    #[allow(clippy::comparison_chain)]
    pub fn new(node: &Node, ptr: Ptr) -> Self {
        let mut rects = vec![];
        let mut text = vec![];
        let mut input_points = vec![];
        let wx: i32;
        let mut wy = 0;
        if let Op::Literal(ref lit) = node.op {
            let l = lit.const_as_ref();
            let repr = if l.is_zero() {
                "zero".to_owned()
            } else if l.is_uone() {
                "uone".to_owned()
            } else if l.is_umax() {
                "umax".to_owned()
            } else if l.is_imax() {
                "imax".to_owned()
            } else if l.is_imin() {
                "imin".to_owned()
            } else {
                format!("0x{}", ExtAwi::bits_to_string_radix(l, false, 16, false, 0).unwrap())
            };
            wx = FONT_WX * (repr.len() as i32);
            wy += 2*FONT_WY + INPUT_FONT_WY;
            let bw = format!("u{}", l.bw());
            text.push(((wx - ((bw.len() as i32) * FONT_WX), FONT_WY + PAD), FONT_SIZE, bw));
            text.push(((0, wy - PAD), FONT_SIZE, repr));
        } else {
            let operation_name = node.op.operation_name();
            let operand_names = node.op.operand_names();
            let total_operand_len: i32 = operand_names.iter().map(|name| name.len() as i32).sum();
            let operation_len = operation_name.len() as i32;
            let min_operands_wx =
                INPUT_FONT_WX * total_operand_len + (2 * PAD) * (operand_names.len() as i32);
            let min_operation_wx = FONT_WX * operation_len + 2 * PAD;
            wx = max(min_operands_wx, min_operation_wx);
            // for spreading out inputs
            let extra_space = if min_operands_wx < min_operation_wx {
                min_operation_wx - min_operands_wx
            } else {
                0
            };
            if operand_names.len() > 1 {
                wy += 2 * INPUT_PAD + INPUT_FONT_WY;
                let individual_spaces = extra_space / (operand_names.len() as i32 - 1);
                let mut x_progression = 0;
                for (op_i, name) in operand_names.iter().enumerate() {
                    text.push(((x_progression + PAD, wy), INPUT_FONT_SIZE, (*name).to_owned()));
                    let this_wx = INPUT_FONT_WX * (name.len() as i32) + 2 * PAD;
                    let center_x = x_progression + (this_wx / 2);
                    input_points.push(((center_x, 0), node.ops[op_i]));
                    x_progression += this_wx + individual_spaces;
                }
            } else if operand_names.len() == 1 {
                input_points.push(((wx / 2, 0), node.ops[0]));
            }
            wy += 2 * PAD + FONT_WY;
            let rect = (0, 0, wx, wy);
            text.push((
                (
                    (wx / 2) - ((FONT_WX * (operation_name.len() as i32)) / 2),
                    wy - PAD,
                ),
                FONT_SIZE,
                operation_name.to_owned(),
            ));
            rects.push(rect);
        }

        Self {
            ptr,
            rects,
            text,
            input_points,
            output_point: (wx / 2, wy),
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
            point.0 .0 += mov.0;
            point.0 .1 += mov.1;
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
    // compress vertically as much as possible
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

    // Reordering columns to try and minimize dependency line crossings.
    // create some maps first.
    let mut ptr_to_x_i: HashMap<Ptr, usize> = HashMap::new();
    let mut x_i_to_ptr: HashMap<usize, Ptr> = HashMap::new();
    for (x_i, vertical) in grid.iter().enumerate() {
        for slot in vertical {
            ptr_to_x_i.insert(slot.0, x_i);
            x_i_to_ptr.insert(x_i, slot.0);
        }
    }
    let mut done_lineages: HashSet<usize> = HashSet::new();
    let mut sorted_lineages: Vec<usize> = vec![];
    // ordered DFS starting from leaves to determine order of lineages
    // done nodes
    let mut done_nodes: HashSet<Ptr> = HashSet::new();
    // frontier
    let mut node: Vec<Ptr> = vec![];
    // path through operands
    let mut ops_i: Vec<usize> = vec![];
    for leaf in &dag.leaves() {
        node.push(*leaf);
        ops_i.push(0);
        loop {
            let current = node[node.len() - 1];
            match dag.dag[current].ops.get(ops_i[ops_i.len() - 1]) {
                Some(operand) => {
                    if done_nodes.contains(operand) {
                        // if node was already explored, check the next dependency
                        let len = ops_i.len();
                        ops_i[len - 1] += 1;
                    } else {
                        // else explore further
                        node.push(*operand);
                        ops_i.push(0);
                    }
                }
                None => {
                    // no more dependents, backtrack
                    let x_i = ptr_to_x_i[&current];
                    if !done_lineages.contains(&x_i) {
                        sorted_lineages.push(x_i);
                        done_lineages.insert(x_i);
                    }
                    done_nodes.insert(current);
                    node.pop();
                    ops_i.pop();
                    if node.is_empty() {
                        break
                    }
                    // check next dependency
                    let len = ops_i.len();
                    ops_i[len - 1] += 1;
                }
            }
        }
    }
    // do the sorting
    let mut new_grid = vec![];
    for x_i in sorted_lineages {
        new_grid.push(grid[x_i].clone());
    }
    let grid = new_grid;

    // find maximum column and row widths, and cumulative positions
    let mut max_y_nodes = 0;
    for vert in &grid {
        for slot in &*vert {
            max_y_nodes = max(max_y_nodes, slot.1 + 1);
        }
    }
    let mut grid_max_wx = vec![0; grid.len()];
    let mut grid_max_wy = vec![0; max_y_nodes];
    let mut rect_grid: Vec<Vec<Option<RenderNode>>> = vec![];
    for _ in 0..grid_max_wx.len() {
        let mut v = vec![];
        for _ in 0..grid_max_wy.len() {
            v.push(None);
        }
        rect_grid.push(v);
    }
    for (x_i, vertical) in grid.iter().enumerate() {
        for (ptr, pos) in &*vertical {
            let node = RenderNode::new(&dag.dag[ptr], *ptr);
            let tmp = grid_max_wx[x_i];
            grid_max_wx[x_i] = max(tmp, node.wx);
            // y is reversed to make the data flow downwards graphically
            let tmp = grid_max_wy[max_y_nodes - 1 - *pos];
            grid_max_wy[max_y_nodes - 1 - *pos] = max(tmp, node.wy);
            rect_grid[x_i][max_y_nodes - 1 - *pos] = Some(node);
        }
    }
    // `rect_grid` is now completely rectangular and the `_max_` values are
    // correct

    // create map for positions in grid
    let mut grid_map: HashMap<Ptr, (usize, usize)> = HashMap::new();
    for (i, vert) in rect_grid.iter().enumerate() {
        for (j, node) in vert.iter().enumerate() {
            // do not flatten, it messes up the indexing
            if let Some(node) = node {
                grid_map.insert(node.ptr, (i, j));
            }
        }
    }

    let mut cumulative_x = vec![];
    let mut tot_wx = 0;
    for wx in &grid_max_wx {
        cumulative_x.push(tot_wx);
        tot_wx += wx + NODE_PAD;
    }
    let mut cumulative_y = vec![];
    let mut tot_wy = 0;
    for wy in &grid_max_wy {
        cumulative_y.push(tot_wy);
        tot_wy += wy + NODE_PAD;
    }

    // translate to final places
    for (i, vert) in rect_grid.iter_mut().enumerate() {
        for (j, node) in vert.iter_mut().enumerate() {
            // do not flatten, it messes up the indexing
            if let Some(node) = node {
                node.translate((cumulative_x[i], cumulative_y[j]));
            }
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

    //s += &format!("<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"#f0f\"/>", tmp.0
    // .0, tmp.0 .1);

    // rectangles and outlines first
    for vert in &rect_grid {
        for node in (*vert).iter().flatten() {
            for rect in &node.rects {
                s += &format!(
                    "<rect fill=\"#111\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>\n",
                    rect.0, rect.1, rect.2, rect.3
                );
                // outline the rectangle
                s += &format!(
                    "<polyline stroke=\"#777\" stroke-width=\"{}\" points=\"{},{} {},{} {},{} \
                     {},{} {},{}\"  fill=\"#0000\"/>\n",
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
    for i in 0..rect_grid.len() {
        for j in 0..rect_grid[i].len() {
            let num_inputs = if let Some(ref node) = rect_grid[i][j] {
                node.input_points.len()
            } else {
                continue
            };
            for k in 0..num_inputs {
                let (i, ptr) = rect_grid[i][j].as_ref().unwrap().input_points[k];
                let (o_i, o_j) = grid_map[&ptr];
                let color = COLORS[o_i % COLORS.len()];
                let o = rect_grid[o_i][o_j].as_ref().unwrap().output_point;
                let p = NODE_PAD / 2;
                s += &format!(
                    "<path stroke=\"#{}\" fill=\"#0000\" d=\"M {},{} C {},{} {},{} {},{}\"/>\n",
                    color,
                    o.0,
                    o.1,
                    o.0,
                    o.1 + p,
                    i.0,
                    i.1 - p,
                    i.0,
                    i.1
                );
            }
        }
    }

    // text last
    for vert in &rect_grid {
        for node in (*vert).iter().flatten() {
            for tmp in &node.text {
                let size = tmp.1;
                s += &format!(
                    "<text fill=\"#777\" font-size=\"{}\" font-family=\"{}\" x=\"{}\" \
                     y=\"{}\">{}</text>\n",
                    size,
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
        <svg preserveAspectRatio=\"meet\" viewBox=\"{} {} {} {}\" width=\"100%\" height=\"100%\" \
        version=\"1.1\" xmlns=\"http://www.w3.org/2000/svg\">\n\
        {}\n\
        </svg>",
        -PAD,
        -PAD,
        viewbox.0 + PAD,
        viewbox.1 + PAD,
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
