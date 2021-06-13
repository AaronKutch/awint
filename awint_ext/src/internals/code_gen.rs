use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use core::num::NonZeroUsize;

use ComponentType::*;

use crate::{
    internals::{parse::parse_concats, structs::*},
    ExtAwi,
};

/// Code generation for concatenations of components macros
pub fn code_gen(input: &str, inlawi: bool, return_source: bool) -> Result<String, String> {
    let concats = match parse_concats(input) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    // index to if we find a concatenation with at least a dynamically determined
    // width
    let mut dynamic_width_i: Option<usize> = None;
    // index to if we find a concatenation with a statically determined width
    let mut static_width_i: Option<(usize, NonZeroUsize)> = None;
    // records what alignments of unbounded ranges we have seen, (lsb, middle, msb)
    let mut unbounded_alignment = (false, false, false);
    for (j0, concat) in concats.iter().enumerate() {
        let mut deterministic_width = true;
        for (j1, component) in concat.concatenation.iter().enumerate() {
            if let Filler = component.component_type {
                if (j0 == 0) && return_source {
                    return Err("a construction macro cannot have a filler in the source \
                                concatenation"
                        .to_owned())
                }
                if component.range.end.is_none() {
                    if concat.concatenation.len() == 1 {
                        // simplify the unbounded alignment logic
                        return Err(format!(
                            "concatenation {} consists of only an unbounded filler",
                            j0,
                        ))
                    }
                    if j1 == 0 {
                        unbounded_alignment.0 = true;
                    }
                    if j1 == (concat.concatenation.len() - 1) {
                        unbounded_alignment.2 = true;
                    }
                    if (j1 != 0) && (j1 != (concat.concatenation.len() - 1)) {
                        unbounded_alignment.1 = true;
                    }
                    deterministic_width = false;
                }
            }
        }
        if let Some((j0_orig, width0)) = static_width_i {
            if let Some(width1) = concat.total_bw {
                if width0 != width1 {
                    return Err(format!(
                        "determined statically that concatenations {} and {} have unequal \
                         bitwidths {} and {}",
                        j0_orig, j0, width0, width1
                    ))
                }
            }
        } else if let Some(width1) = concat.total_bw {
            static_width_i = Some((j0, width1));
        } else if deterministic_width && dynamic_width_i.is_none() {
            dynamic_width_i = Some(j0);
        }
    }
    if inlawi && static_width_i.is_none() {
        return Err(
            "InlAwi construction macros need at least one concatenation to have a width that can \
             be determined statically by the macro"
                .to_owned(),
        )
    }
    let undetermined = dynamic_width_i.is_none() && static_width_i.is_none();
    if undetermined && unbounded_alignment.1 {
        return Err(
            "there is an unbounded filler in the middle of a concatenation, and no concatenation \
             has a statically or dynamically determinable width"
                .to_owned(),
        )
    }
    if undetermined && (unbounded_alignment.0 && unbounded_alignment.2) {
        return Err(
            "there are two concatenations with unbounded fillers aligned opposite each other, and \
             no concatenation has a statically or dynamically determinable width"
                .to_owned(),
        )
    }
    let total_bw = if let Some((_, bw)) = static_width_i {
        Some(bw)
    } else {
        None
    };

    // code gen

    // first check for infallibility
    if (concats.len() == 1) && (concats[0].concatenation.len() == 1) {
        let comp = &concats[0].concatenation[0];
        if let Literal(ref lit) = comp.component_type {
            // constants have been normalized by now
            if comp.range.static_width().is_some() {
                // Return one constant
                if inlawi {
                    return Ok(format!(
                        "InlAwi::<{}, {}>::unstable_from_slice(&{:?})",
                        lit.bw(),
                        lit.raw_len(),
                        lit[..].as_raw_slice(),
                    ))
                } else {
                    return Ok(format!(
                        "ExtAwi::from_bits(InlAwi::<{}, \
                         {}>::unstable_from_slice(&{:?}).const_as_ref())",
                        lit.bw(),
                        lit.raw_len(),
                        lit[..].as_raw_slice(),
                    ))
                }
            }
        }
    }

    let constants = code_gen_constants(&concats[0], total_bw);
    let component_checks = code_gen_component_checks(&concats);
    let common_bw = code_gen_common_bw(&concats, dynamic_width_i, total_bw);
    let concats_checks = code_gen_concats_checks(&concats);

    // before moving on to the final fielding or copying steps, handle special cases

    // check for plain copy `a[..]; b[..]; c[..]; ...` cases
    let mut plain_copy = true;
    for concat in &concats {
        if concat.concatenation.len() != 1 {
            plain_copy = false;
            break
        }
    }

    // check if all ranges are static
    let mut all_static_ranges = true;
    'outer: for concat in &concats {
        for comp in &concat.concatenation {
            if comp.range.static_range().is_none() {
                all_static_ranges = false;
                break 'outer
            }
        }
    }

    let constructing = if return_source {
        if inlawi {
            let bw = total_bw.unwrap();
            let raw_len = ExtAwi::zero(bw).raw_len();
            format!("let {} = InlAwi::<{}, {}>::zero();\n", RET_AWI, bw, raw_len)
        } else {
            format!("let {} = ExtAwi::panicking_zero({});\n", RET_AWI, BW)
        }
    } else if plain_copy || all_static_ranges {
        String::new()
    } else {
        // need a temporary, `RET_AWI` is not actually returned
        if let Some(bw) = total_bw {
            let raw_len = ExtAwi::zero(bw).raw_len();
            format!("let {} = InlAwi::<{}, {}>::zero();\n", RET_AWI, bw, raw_len)
        } else {
            format!("let {} = ExtAwi::panicking_zero({});\n", RET_AWI, BW)
        }
    };

    let fielding = if plain_copy {
        code_gen_plain_copy(&concats, total_bw, inlawi, return_source)
    } else if all_static_ranges {
        // TODO static fielding
        String::new()
    } else {
        code_gen_dynamic_fielding(&concats, total_bw, inlawi, return_source)
    };

    let returning = if return_source {
        format!("Some({})", RET_AWI)
    } else {
        "Some(())".to_owned()
    };

    // The layout is like:
    // constants // define `InlAwi` constants
    // if component_checks {None} // checks that need to run before the common
    // bitwidth is computed else {
    //     common_bw // calculate the common bitwidth
    //     if concats_checks {None} // check that concatenation bitwidths equal the
    //                              // common bitwidth
    //     else {
    //         constructing // construction
    //         fielding // copying or fielding
    //         returning // returning
    //     }
    // }
    match (component_checks.is_empty(), concats_checks.is_empty()) {
        (false, false) => Ok(format!(
            "{{\n{}if\n{}\n{{None}} else {{\n{}\nif\n{}\n{{None}} else {{\n{}\n{}\n{}\n}}\n}}}}",
            constants,
            component_checks,
            common_bw,
            concats_checks,
            constructing,
            fielding,
            returning
        )),
        (false, true) => Ok(format!(
            "{{\n{}if\n{}\n{{None}} else {{\n{}\n{}\n{}\n{}\n}}\n}}",
            constants, component_checks, common_bw, constructing, fielding, returning
        )),
        (true, false) => Ok(format!(
            "{{\n{}\n{}\nif\n{}\n{{None}} else {{\n{}\n{}\n{}\n}}\n}}",
            constants, common_bw, concats_checks, constructing, fielding, returning
        )),
        (true, true) => Ok(format!(
            "{{\n{}\n{}\n{}\n{}\n{}\n}}",
            constants, common_bw, constructing, fielding, returning
        )),
    }
}

/// Generates all `InlAwi` constants needed
fn code_gen_constants(source: &Concatenation, total_bw: Option<NonZeroUsize>) -> String {
    let mut constants = String::new();
    // TODO in some cases it is beneficial to merge constants and zeroed areas for
    // non-literal static width components.
    /*
    let mut lsb_i = 0;
    let mut shl = 0;
    let mut msb_i = source.concatenation.len();
    let mut shr = 0;
    if let Some(total_bw) = total_bw {
        let mut awi = ExtAwi::zero(total_bw);
        // add to the static constant until we reach something dynamic in width
        while lsb_i < source.concatenation.len() {
            let comp = &source.concatenation[lsb_i];
            if let Some(r) = comp.range.static_range() {
                let w = comp.range.static_width().unwrap();
                match comp.component_type {
                    Literal(ref lit) => {
                        awi[..].field(shl, &lit[..], r.0, w).unwrap();
                    }
                    Variable(_) => (),
                    Filler => unreachable!(),
                }
                shl += w;
            } else {
                lsb_i += 1;
                break
            }
            lsb_i += 1;
        }
        // start again from the msb if needed
        while msb_i > lsb_i {
            let comp = &source.concatenation[lsb_i];
            if let Some(r) = comp.range.static_range() {
                let w = comp.range.static_width().unwrap();
                shr += w;
                match comp.component_type {
                    Literal(ref lit) => {
                        awi[..]
                            .field(total_bw.get() - shr, &lit[..], r.0, w)
                            .unwrap();
                    }
                    Variable(_) => (),
                    Filler => unreachable!(),
                }
            } else {
                msb_i -= 1;
                break
            }
            msb_i -= 1;
        }
        constants += &format!(
            "let {} = InlAwi::<{}, {}>::unstable_from_slice(&{:?});\n",
            ?,
            awi.bw(),
            awi.raw_len(),
            awi[..].as_raw_slice(),
        );
    }
    // Now create constants that "float" in the middle because of dynamic ranges.
    // Also includes constants with dynamic ranges on them.
    */

    for i in 0..source.concatenation.len() {
        let comp = &source.concatenation[i];
        if let Literal(ref lit) = comp.component_type {
            constants += &format!(
                "let {}_{} = InlAwi::<{}, {}>::unstable_from_slice(&{:?});\n",
                CONSTANT,
                i,
                lit.bw(),
                lit.raw_len(),
                lit[..].as_raw_slice(),
            );
        }
    }
    constants
}

fn add_partials(partials: Vec<String>) -> String {
    if partials.is_empty() {
        return String::new()
    }
    if partials.len() == 1 {
        return partials[0].clone()
    }
    let mut sum = "(".to_owned();
    for i in 0..(partials.len() - 1) {
        sum += &partials[i];
        sum += " + ";
    }
    sum += &partials[partials.len() - 1];
    sum += ")";
    sum
}

fn or_partials(partials: Vec<String>) -> String {
    if partials.is_empty() {
        return String::new()
    }
    if partials.len() == 1 {
        return partials[0].clone()
    }
    let mut sum = "(".to_owned();
    for i in 0..(partials.len() - 1) {
        sum += &partials[i];
        sum += " || ";
    }
    sum += &partials[partials.len() - 1];
    sum += ")";
    sum
}

fn code_gen_component_checks(concats: &[Concatenation]) -> String {
    let mut partials: Vec<String> = Vec::new();
    for concat in &concats[..] {
        for (i, comp) in concat.concatenation.iter().enumerate() {
            if let Some(s) = comp
                .code_gen_bounds_check(if let Literal(_) = comp.component_type {
                    Some(format!("{}_{}", CONSTANT, i))
                } else {
                    None
                })
                .unwrap()
            {
                partials.push(s);
            }
        }
    }
    or_partials(partials)
}

fn code_gen_common_bw(
    concats: &[Concatenation],
    dynamic_width_i: Option<usize>,
    total_bw: Option<NonZeroUsize>,
) -> String {
    if let Some(bw) = total_bw {
        format!("let {} = {};", BW, bw)
    } else {
        // for dynamic bitwidths, we recorded the index of one concatenation
        // which we know has a runtime deterministic bitwidth. In the case of
        // all unbounded concatenations, we just choose the zeroeth, because of
        // the alignment guarantees.
        let concat = &concats[dynamic_width_i.unwrap_or(0)];
        let mut partials: Vec<String> = Vec::new();
        for (i, comp) in concat.concatenation.iter().enumerate() {
            if let Some(s) = comp.code_gen_bw(if let Literal(_) = comp.component_type {
                Some(format!("{}_{}", CONSTANT, i))
            } else {
                None
            }) {
                partials.push(s);
            } else {
                // do nothing because this must involve the case of aligned
                // unbounded fillers
            }
        }
        format!("let {} = {};", BW, add_partials(partials))
    }
}

/// Generates all the needed checks that run at runtime.
fn code_gen_concats_checks(concats: &[Concatenation]) -> String {
    let mut checks = Vec::new();
    for concat in &concats[..] {
        let mut partials: Vec<String> = Vec::new();
        for (i, comp) in concat.concatenation.iter().enumerate() {
            if let Some(s) = comp.code_gen_bw(if let Literal(_) = comp.component_type {
                Some(format!("{}_{}", CONSTANT, i))
            } else {
                None
            }) {
                partials.push(s);
            } else {
                // do nothing because this must involve the case of aligned
                // unbounded fillers
            }
        }
        checks.push(format!("({} != {})", add_partials(partials), BW));
    }

    or_partials(checks)
}

fn code_gen_plain_copy(
    concats: &[Concatenation],
    total_bw: Option<NonZeroUsize>,
    inlawi: bool,
    return_source: bool,
) -> String {
    let mut fielding = String::new();
    let source = concats[0].concatenation[0].code_gen_name(Some(0));
    for j0 in 1..concats.len() {
        let comp = &concats[j0].concatenation[0];
        let sink = if j0 == 0 {
            if !return_source {
                continue
            }
            RET_AWI.to_owned()
        } else {
            comp.code_gen_name(None)
        };
        fielding += &format!(
            "{}.const_as_mut().copy_assign({}.const_as_ref()).unwrap();",
            sink, source
        );
    }
    fielding
}

fn code_gen_dynamic_fielding(
    concats: &[Concatenation],
    total_bw: Option<NonZeroUsize>,
    inlawi: bool,
    return_source: bool,
) -> String {
    let mut fielding = String::new();

    // TODO need temporary always
    /*for j0 in 1..concats.len() {
        let concat = &concats[j0];
        fielding += &format!("let mut {} = 0;", SHL);
        let target = if j0 == 0 {
            if !return_source {
                continue;
            }
            RET_AWI
        } else {

        }
        for j1 in 0..concat.concatenation.len() {
            let comp = &concat.concatenation[j1];
            let name = comp.code_gen_name(Some(j1));
            fielding += &format!("{}.field({}, {}.const_as_ref(), {}, {});{} += {}.bw();",
            name, SHL, SHL, name);
        }
    }*/

    fielding
}
