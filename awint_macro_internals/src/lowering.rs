//! main lowering logic
//!
//! There are some things to explain about the fielding implementation. If we
//! tried to do something like ORing all the boundaries of the source and sink
//! together, and then fielding all those regions, it would lead to arbitrarily
//! complex logic because dynamic ranges can push the components to align with
//! multiple different components in the sink, and slide to completely different
//! places at runtime. In other words, if all ranges are dynamic then every
//! fielding operation would need to be ready to field any component from the
//! source to any component in the sink. The other problem is that fielding
//! between references pointing to the same awi violates borrow rules. The only
//! reasonable solution is to have an intermediate buffer, to which the source
//! is fielded, then fielding from there to the sinks. This removes the need for
//! the fieldings only needing to know about one concatenation at a time and not
//! needing to account for both at the same time, and the complexity is kept
//! linear
//!
//! No-op fillers in sources adds one more layer of complexity. In most cases,
//! the bits in buffers corresponding to fillers are in a specified set state or
//! are never read from, but consider what happens in this case:
//!
//! (y is set to 0x2222u16 before the operation)
//! `cc!(0x1111u16, ..8; ..8, y)`
//!
//! The output using the above approach would be `0x1100u16` instead of the
//! expected `0x1122u16` (assuming the buffer bits all start out as zeroed).
//! No-op source fillers effectively dictate that sink bits must be fielded to a
//! buffer, then non-filler source bits overwrite their respective parts, and
//! then the buffer gets written back to the sinks. Any other method runs into
//! the complexity multiplication problem, because the dynamic fielding to the
//! sink would need to be aware of all dynamic fillers in the source.

use std::{
    collections::{BinaryHeap, HashMap, HashSet},
    num::NonZeroUsize,
};

use awint_ext::ExtAwi;
use ComponentType::*;

use crate::*;

/// Lowering of the parsed structs into Rust code.
pub(crate) fn lower(
    concats: &[Concatenation],
    dynamic_width_i: Option<usize>,
    total_bw: Option<NonZeroUsize>,
    specified_source: bool,
    undetermined: bool,
    construct_fn: &str,
    inlawi: bool,
    return_source: bool,
) -> String {
    // use hashmaps to eliminate duplication, binary heaps to insure determinism,
    // and create an identifier map for later use of the variables

    // create constants
    let mut literals: HashSet<OrdExtAwi> = HashSet::new();
    for comp in &concats[0].concatenation {
        if let Literal(ref lit) = comp.component_type {
            literals.insert(OrdExtAwi(lit.clone()));
        }
    }
    let ord_literals: BinaryHeap<OrdExtAwi> = literals.drain().collect();
    let num_literals = ord_literals.len();
    let mut literal_to_id: HashMap<ExtAwi, usize> = HashMap::new();
    let mut constants = String::new();
    for (id, lit) in ord_literals.into_iter_sorted().enumerate() {
        constants += &format!(
            "let {}_{} = InlAwi::<{}, {}>::unstable_from_slice(&{:?});\n",
            CONSTANT,
            id,
            lit.0.bw(),
            lit.0.raw_len(),
            lit.0[..].as_raw_slice(),
        );
        literal_to_id.insert(lit.0, id);
    }

    // track all the other values we will need
    let mut values: HashSet<String> = HashSet::new();
    // because optimizations in the value lowering eliminate 0
    values.insert("0".to_owned());
    for concat in concats {
        for comp in &concat.concatenation {
            lower_values(&mut values, &literal_to_id, comp);
        }
    }
    // get determinism
    let ord_values: BinaryHeap<String> = values.drain().collect();
    // mapping values to unique ids
    let mut value_to_id: HashMap<String, usize> = HashMap::new();
    for (id, value) in ord_values.into_iter_sorted().enumerate() {
        value_to_id.insert(value, id);
    }
    // this will keep track of what values are actually used
    let mut used_values: HashSet<String> = HashSet::new();

    // Create components bounds checks
    let mut comp_checks: HashSet<(String, String)> = HashSet::new();
    for concat in concats {
        for comp in &concat.concatenation {
            lower_component_checks(&mut comp_checks, &literal_to_id, comp);
        }
    }
    let ord_comp_checks: BinaryHeap<(String, String)> = comp_checks.drain().collect();
    let mut comp_check_partials: Vec<String> = Vec::new();
    for check in ord_comp_checks.into_iter_sorted() {
        let id0 = value_to_id[&check.0];
        let id1 = value_to_id[&check.1];
        // push a less-than check
        comp_check_partials.push(format!("({}_{}, {}_{})", VALUE, id0, VALUE, id1));
        used_values.insert(check.0.clone());
        used_values.insert(check.1.clone());
    }

    // track widths which will be used for concat checks and fielding
    let mut widths: HashSet<Width> = HashSet::new();
    for concat in concats {
        for comp in &concat.concatenation {
            if let Some(width) = lower_width(Some(&literal_to_id), comp) {
                widths.insert(width);
            }
        }
    }
    let ord_widths: BinaryHeap<Width> = widths.drain().collect();
    // mapping widths to unique ids
    let mut width_to_id: HashMap<Width, usize> = HashMap::new();
    for (id, width) in ord_widths.into_iter_sorted().enumerate() {
        width_to_id.insert(width, id);
    }
    // this will keep track of what widths are actually used
    let mut used_widths: HashSet<Width> = HashSet::new();

    // create concatenation bounds checks
    let mut concat_lt_partials = Vec::new();
    let mut concat_ne_partials = Vec::new();
    let mut s_bitwidths = String::new();
    let mut bitwidth_partials = Vec::new();
    for (id, concat) in concats.iter().enumerate() {
        let mut partials: Vec<String> = Vec::new();
        let mut unbounded = false;
        for comp in &concat.concatenation {
            if let Some(width) = lower_width(Some(&literal_to_id), comp) {
                partials.push(format!("{}_{}", WIDTH, width_to_id[&width]));
                used_widths.insert(width.clone());
            } else {
                unbounded = true;
            }
        }
        if partials.is_empty() {
            continue
        }
        let name = format!("{}_{}", BW, id);
        s_bitwidths += &format!("let {} = {};\n", name, add_partials(partials));
        if unbounded {
            // check that we aren't trying to squeeze the unbounded filler into negative
            // widths
            if undetermined {
                // there should be no concat checks, and we need these for the common bitwidth
                // calculation
                bitwidth_partials.push(name);
            } else {
                concat_lt_partials.push(name);
            }
        } else {
            // check that the concatenation width is equal to the common bitwidth
            concat_ne_partials.push(name);
        }
    }

    // create the common bitwidth
    let common_bw = if let Some(bw) = total_bw {
        // note: need `: usize` because of this case
        format!("let {}: usize = {};\n", BW, bw)
    } else if undetermined {
        // for the case with all unbounded fillers, find the max bitwidth for the buffer
        // to use.
        format!(
            "let {}: usize = Bits::unstable_max({});\n",
            BW,
            array_partials(bitwidth_partials)
        )
    } else {
        // for dynamic bitwidths, we recorded the index of one concatenation
        // which we know has a runtime deterministic bitwidth. In the case of
        // all unbounded concatenations, we just choose the zeroeth concatenation
        let id = dynamic_width_i.unwrap_or(0);
        let name = format!("{}_{}", BW, id);
        let s = format!("let {}: usize = {};\n", BW, name);
        s
    };

    // create all references we may need
    let mut refs: HashSet<String> = HashSet::new();
    for concat in concats {
        for comp in &concat.concatenation {
            if let Some(name) = lowered_name(Some(&literal_to_id), comp) {
                refs.insert(name.clone());
            }
        }
    }
    let mut ref_to_id: HashMap<String, usize> = HashMap::new();
    let ord_refs: BinaryHeap<String> = refs.drain().collect();
    for (id, var) in ord_refs.into_iter_sorted().enumerate() {
        ref_to_id.insert(var, id + num_literals);
    }
    // for immutable refs
    let mut used_ref_refs: HashSet<String> = HashSet::new();
    // for mutable refs
    let mut used_mut_refs: HashSet<String> = HashSet::new();

    // check for simplest copy `a[..]; b[..]; c[..]; ...` cases
    let mut all_copy_assign = true;
    for concat in concats {
        if (concat.concatenation.len() != 1) || !concat.concatenation[0].has_full_range() {
            all_copy_assign = false;
            break
        }
    }

    // true if the input is of the form
    // `constant; a[..]; b[..]; c[..]; ...` or
    // `single full range var; a[..]; b[..]; c[..]; ...`
    let no_buffer = all_copy_assign
        && (concats[0].concatenation.len() == 1)
        && concats[0].concatenation[0].has_full_range();

    let mut constructing = if return_source {
        if inlawi {
            let bw = total_bw.unwrap();
            let raw_len = ExtAwi::zero(bw).raw_len();
            format!(
                "let mut {} = InlAwi::<{}, {}>::{}();\n",
                AWI, bw, raw_len, construct_fn
            )
        } else {
            // even if the bitwidth is known statically, we return `ExtAwi` from `extawi!`
            format!(
                "let mut {} = ExtAwi::panicking_{}({});\n",
                AWI, construct_fn, BW
            )
        }
    } else if no_buffer {
        String::new()
    } else {
        // still need a temporary, `AWI` is not actually returned
        if let Some(bw) = total_bw {
            let raw_len = ExtAwi::zero(bw).raw_len();
            format!(
                "let mut {} = InlAwi::<{}, {}>::{}();\n",
                AWI, bw, raw_len, construct_fn
            )
        } else {
            format!(
                "let mut {} = ExtAwi::panicking_{}({});\n",
                AWI, construct_fn, BW
            )
        }
    };
    if !no_buffer {
        constructing += &format!("let {} = {}.const_as_mut();\n", AWI_REF, AWI);
    }

    let mut filler_in_source = false;
    for comp in &concats[0].concatenation {
        if let Filler = comp.component_type {
            filler_in_source = true;
            break
        }
    }

    let mut fielding = String::new();

    if filler_in_source && !specified_source {
        // note: in all cases that reach here the source must be `AWI_REF`
        for j0 in 1..concats.len() {
            let concat = &concats[j0];
            let use_copy_assign =
                concat.concatenation.len() == 1 && concat.concatenation[0].has_full_range();
            // see notes at top of file
            // sink -> buffer
            if use_copy_assign {
                // use copy assign
                let sink_comp = &concat.concatenation[0];
                if let Some(sink_name) = lowered_name(Some(&literal_to_id), sink_comp) {
                    fielding += &format!(
                        "{}.const_as_mut().copy_assign({}_{}).unwrap();\n",
                        AWI_REF.to_owned(),
                        REF,
                        ref_to_id[&sink_name]
                    );
                    used_ref_refs.insert(sink_name);
                } // else it is a no-op filler
            } else {
                fielding += &lower_fielding_to_awi(
                    &mut used_values,
                    &mut used_widths,
                    &mut used_ref_refs,
                    &literal_to_id,
                    &value_to_id,
                    &width_to_id,
                    &ref_to_id,
                    concat,
                );
            }
            // no general possibilty for copy assigning, because there is a sink
            // source -> buffer
            fielding += &lower_fielding_to_awi(
                &mut used_values,
                &mut used_widths,
                &mut used_ref_refs,
                &literal_to_id,
                &value_to_id,
                &width_to_id,
                &ref_to_id,
                &concats[0],
            );
            // buffer -> sink
            if use_copy_assign {
                // use copy assign
                let sink_comp = &concat.concatenation[0];
                if let Some(sink_name) = lowered_name(Some(&literal_to_id), sink_comp) {
                    fielding += &format!(
                        "{}_{}.const_as_mut().copy_assign({}).unwrap();\n",
                        REF,
                        ref_to_id[&sink_name],
                        AWI_REF.to_owned()
                    );
                    used_mut_refs.insert(sink_name);
                }
            } else {
                fielding += &lower_fielding_from_awi(
                    &mut used_values,
                    &mut used_widths,
                    &mut used_mut_refs,
                    &value_to_id,
                    &width_to_id,
                    &ref_to_id,
                    concat,
                );
            }
        }
    } else if no_buffer {
        let name = lowered_name(Some(&literal_to_id), &concats[0].concatenation[0]).unwrap();
        let source_name = format!("{}_{}", REF, ref_to_id[&name]);
        used_ref_refs.insert(name);
        // simplest copy assigning
        for concat in &concats[1..] {
            let sink_comp = &concat.concatenation[0];
            if let Some(sink_name) = lowered_name(Some(&literal_to_id), sink_comp) {
                fielding += &format!(
                    "{}_{}.const_as_mut().copy_assign({}).unwrap();\n",
                    REF, ref_to_id[&sink_name], source_name
                );
                used_mut_refs.insert(sink_name);
            }
        }
    } else {
        // source -> buffer once
        if concats[0].concatenation.len() == 1 && concats[0].concatenation[0].has_full_range() {
            let sink_comp = &concats[0].concatenation[0];
            if let Some(sink_name) = lowered_name(Some(&literal_to_id), sink_comp) {
                fielding += &format!(
                    "{}.const_as_mut().copy_assign({}_{}).unwrap();\n",
                    AWI_REF.to_owned(),
                    REF,
                    ref_to_id[&sink_name]
                );
                used_ref_refs.insert(sink_name);
            }
        } else {
            fielding += &lower_fielding_to_awi(
                &mut used_values,
                &mut used_widths,
                &mut used_ref_refs,
                &literal_to_id,
                &value_to_id,
                &width_to_id,
                &ref_to_id,
                &concats[0],
            );
        }
        // buffer -> sinks
        for concat in &concats[1..] {
            if concat.concatenation.len() == 1 && concat.concatenation[0].has_full_range() {
                let sink_comp = &concat.concatenation[0];
                if let Some(sink_name) = lowered_name(Some(&literal_to_id), sink_comp) {
                    fielding += &format!(
                        "{}_{}.const_as_mut().copy_assign({}).unwrap();\n",
                        REF,
                        ref_to_id[&sink_name],
                        AWI_REF.to_owned()
                    );
                    used_mut_refs.insert(sink_name);
                }
            } else {
                fielding += &lower_fielding_from_awi(
                    &mut used_values,
                    &mut used_widths,
                    &mut used_mut_refs,
                    &value_to_id,
                    &width_to_id,
                    &ref_to_id,
                    concat,
                );
            }
        }
    }

    // Lower all used widths by calculating them. This uses more values, which is
    // why this must run first. Overflow is not possible because of component
    // checks.
    let ord_used_widths: BinaryHeap<Width> = used_widths.drain().collect();
    let mut s_widths = String::new();
    for width in ord_used_widths.into_iter_sorted() {
        match width {
            Width::Single(ref s) => {
                used_values.insert(s.clone());
                s_widths += &format!(
                    "let {}_{} = {}_{};\n",
                    WIDTH, width_to_id[&width], VALUE, value_to_id[s]
                );
            }
            Width::Range(ref s0, ref s1) => {
                used_values.insert(s0.clone());
                used_values.insert(s1.clone());
                s_widths += &format!(
                    "let {}_{} = {}_{}.wrapping_sub({}_{});\n",
                    WIDTH, width_to_id[&width], VALUE, value_to_id[s1], VALUE, value_to_id[s0]
                );
            }
        }
    }

    // lower all used values by assigning them to `let` bindings
    let ord_used_values: BinaryHeap<String> = used_values.drain().collect();
    let mut s_values = String::new();
    for val in ord_used_values.into_iter_sorted() {
        s_values += &format!("let {}_{}: usize = {};\n", VALUE, value_to_id[&val], val);
    }

    // lower all used references by assigning them to `let` bindings
    let ord_used_mut_refs: BinaryHeap<String> = used_mut_refs.drain().collect();
    let mut referencing = String::new();
    for reference in ord_used_mut_refs.into_iter_sorted() {
        // mutable bindings supersede immutable ones
        used_ref_refs.remove(&reference);
        referencing += &format!(
            "let {}_{}: &mut Bits = {}.const_as_mut();\n",
            REF, ref_to_id[&reference], reference
        );
    }
    let ord_used_ref_refs: BinaryHeap<String> = used_ref_refs.drain().collect();
    for reference in ord_used_ref_refs.into_iter_sorted() {
        referencing += &format!(
            "let {}_{}: &Bits = {}.const_as_ref();\n",
            REF, ref_to_id[&reference], reference
        );
    }

    let returning = if undetermined && comp_check_partials.is_empty() {
        // infallible
        String::new()
    } else if return_source {
        format!("Some({})", AWI)
    } else {
        "Some(())".to_owned()
    };

    // construct the output code by starting with the innermost scope
    let mut output = format!(
        "\n{}\n{}\n{}\n{}\n",
        referencing, constructing, fielding, returning
    );
    match (concat_ne_partials.is_empty(), concat_lt_partials.is_empty()) {
        (true, true) => (),
        (true, false) => {
            output = format!(
                "if Bits::unstable_common_lt_checks({}, {}).is_some() {{\n{}\n}} else {{None}}",
                BW,
                array_partials(concat_lt_partials),
                output
            )
        }
        (false, true) => {
            output = format!(
                "if Bits::unstable_common_ne_checks({}, {}).is_some() {{\n{}\n}} else {{None}}",
                BW,
                array_partials(concat_ne_partials),
                output
            )
        }
        (false, false) => {
            output = format!(
                "if Bits::unstable_common_ne_checks({}, {}).is_some()\n&& \
                 Bits::unstable_common_lt_checks({}, {}).is_some() {{\n{}\n}} else {{None}}",
                BW,
                array_partials(concat_ne_partials),
                BW,
                array_partials(concat_lt_partials),
                output
            )
        }
    }
    output = format!("{}\n{}\n{}\n{}", s_widths, s_bitwidths, common_bw, output);
    if !comp_check_partials.is_empty() {
        output = format!(
            "if Bits::unstable_lt_checks({}).is_some() {{\n{}\n}} else {{None}}",
            array_partials(comp_check_partials),
            output
        );
    }
    output = format!("{{\n{}\n{}\n{}\n}}", constants, s_values, output);
    output
}