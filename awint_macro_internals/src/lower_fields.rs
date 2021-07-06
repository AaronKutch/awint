//! Code responsible for lowering fielding ops

use std::collections::{HashMap, HashSet};

use awint_ext::ExtAwi;

use crate::*;

fn field_to_awi(
    used_values: &mut HashSet<String>,
    used_widths: &mut HashSet<Width>,
    used_ref_refs: &mut HashSet<String>,
    literal_to_id: &HashMap<ExtAwi, usize>,
    value_to_id: &HashMap<String, usize>,
    width_to_id: &HashMap<Width, usize>,
    ref_to_id: &HashMap<String, usize>,
    comp: &Component,
    width: Width,
    other_align: bool,
) -> String {
    let mut field = String::new();
    let s_width = format!("{}_{}", WIDTH, width_to_id[&width]);
    if other_align {
        // subtract the `SHL` amount first
        field += &format!("{} -= {};", SHL, s_width);
    }
    if let Some(comp_name) = lowered_name(Some(literal_to_id), comp) {
        let start_val = comp.range.start.clone().unwrap().code_gen_value();
        // fields `s_width` bits of from `start_val` in `comp_name` to `SHL` in
        // `AWI_REF`.
        field += &format!(
            "{}.field({}, {}_{}, {}_{}, {}).unwrap();",
            AWI_REF, SHL, REF, ref_to_id[&comp_name], VALUE, value_to_id[&start_val], s_width,
        );
        used_ref_refs.insert(comp_name);
        used_values.insert(start_val);
    } // else is a filler
      // this runs for both fillers and nonfillers
    if other_align {
        field += "\n";
    } else {
        // add to the `SHL` afterwards
        field += &format!("{} += {};\n", SHL, s_width);
    }
    used_widths.insert(width);
    field
}

pub(crate) fn lower_fielding_to_awi(
    used_values: &mut HashSet<String>,
    used_widths: &mut HashSet<Width>,
    used_ref_refs: &mut HashSet<String>,
    literal_to_id: &HashMap<ExtAwi, usize>,
    value_to_id: &HashMap<String, usize>,
    width_to_id: &HashMap<Width, usize>,
    ref_to_id: &HashMap<String, usize>,
    concat: &Concatenation,
) -> String {
    let mut fielding = String::new();
    // Construct the value of `AWI`.
    fielding += &format!("let mut {}: usize = 0;\n", SHL);
    let mut lsb_i = 0;
    while lsb_i < concat.concatenation.len() {
        let comp = &concat.concatenation[lsb_i];
        if let Some(width) = lower_width(Some(literal_to_id), comp) {
            fielding += &field_to_awi(
                used_values,
                used_widths,
                used_ref_refs,
                literal_to_id,
                value_to_id,
                width_to_id,
                ref_to_id,
                comp,
                width,
                false,
            );
        } else {
            // if we encounter an unbounded filler, reset and try again from the most
            // significant side
            break
        };
        lsb_i += 1;
    }
    let mut msb_i = concat.concatenation.len() - 1;
    if msb_i > lsb_i {
        fielding += &format!("let mut {}: usize = {};\n", SHL, BW);
    }
    while msb_i > lsb_i {
        let comp = &concat.concatenation[msb_i];
        let width = lower_width(Some(literal_to_id), comp).unwrap();
        fielding += &field_to_awi(
            used_values,
            used_widths,
            used_ref_refs,
            literal_to_id,
            value_to_id,
            width_to_id,
            ref_to_id,
            comp,
            width,
            true,
        );
        msb_i -= 1;
    }
    fielding
}

fn field_from_awi(
    used_values: &mut HashSet<String>,
    used_widths: &mut HashSet<Width>,
    used_mut_refs: &mut HashSet<String>,
    value_to_id: &HashMap<String, usize>,
    width_to_id: &HashMap<Width, usize>,
    ref_to_id: &HashMap<String, usize>,
    comp: &Component,
    width: Width,
    other_align: bool,
) -> String {
    let mut field = String::new();
    let s_width = format!("{}_{}", WIDTH, width_to_id[&width]);
    if other_align {
        // subtract the `SHL` amount first
        field += &format!("{} -= {};", SHL, s_width);
    }
    if let Some(comp_name) = lowered_name(None, comp) {
        let start_val = comp.range.start.clone().unwrap().code_gen_value();
        // fields `s_width` bits of from `start_val` in `AWI_REF` to `SHL` in
        // `comp_name`.
        field += &format!(
            "{}_{}.field({}_{}, {}, {}, {}).unwrap();",
            REF, ref_to_id[&comp_name], VALUE, value_to_id[&start_val], AWI_REF, SHL, s_width,
        );
        used_mut_refs.insert(comp_name);
        used_values.insert(start_val);
    } // else is a filler
      // this runs for both fillers and nonfillers
    if other_align {
        field += "\n";
    } else {
        // add to the `SHL` afterwards
        field += &format!("{} += {};\n", SHL, s_width);
    }
    used_widths.insert(width);
    field
}

// Assumes that `AWI` has been constructed and can be used as the source.
pub(crate) fn lower_fielding_from_awi(
    used_values: &mut HashSet<String>,
    used_widths: &mut HashSet<Width>,
    used_mut_refs: &mut HashSet<String>,
    value_to_id: &HashMap<String, usize>,
    width_to_id: &HashMap<Width, usize>,
    ref_to_id: &HashMap<String, usize>,
    concat: &Concatenation,
) -> String {
    let mut fielding = String::new();
    fielding += &format!("let mut {}: usize = 0;\n", SHL);
    let mut lsb_i = 0;
    while lsb_i < concat.concatenation.len() {
        let comp = &concat.concatenation[lsb_i];
        // if we encounter an unbounded filler, reset and try again from the most
        // significant side
        if let Some(width) = lower_width(None, comp) {
            fielding += &field_from_awi(
                used_values,
                used_widths,
                used_mut_refs,
                value_to_id,
                width_to_id,
                ref_to_id,
                comp,
                width,
                false,
            );
        } else {
            break
        };
        lsb_i += 1;
    }
    let mut msb_i = concat.concatenation.len() - 1;
    if msb_i > lsb_i {
        fielding += &format!("let mut {}: usize = {};\n", SHL, BW);
    }
    while msb_i > lsb_i {
        let comp = &concat.concatenation[msb_i];
        let width = lower_width(None, comp).unwrap();
        fielding += &field_from_awi(
            used_values,
            used_widths,
            used_mut_refs,
            value_to_id,
            width_to_id,
            ref_to_id,
            comp,
            width,
            true,
        );
        msb_i -= 1;
    }
    fielding
}
