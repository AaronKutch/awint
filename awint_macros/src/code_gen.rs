use std::num::NonZeroUsize;

use awint_core::{bw, Bits};
use awint_ext::ExtAwi;
use ComponentType::*;

use crate::structs::*;

const PREFIX: &str = "awint_macro_generated";
const RETURN_AWI: &str = "awint_macro_generated_return_awi";
const DYNAMIC_BW: &str = "awint_macro_generated_dynamic_bw";

pub(crate) fn code_gen(
    input: &str,
    static_width_source: bool,
    return_source: bool,
) -> Result<String, String> {
    let concats = match crate::parse_concats(input) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };

    // index to if we find a concatenation with at least a dynamically determined
    // width
    let mut dynamic_width: Option<usize> = None;
    // index to if we find a concatenation with a statically determined width
    let mut static_width: Option<(usize, NonZeroUsize)> = None;
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
        if let Some((j0_orig, width0)) = static_width {
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
            static_width = Some((j0, width1));
        } else if deterministic_width {
            // gets the last known dynamic width. this value is not used if `static_width`
            // is set
            dynamic_width = Some(j0);
        }
    }
    if static_width_source && static_width.is_none() {
        return Err(
            "InlAwi construction macros need at least one concatenation to have a width that can \
             be determined statically by the macro"
                .to_owned(),
        )
    }
    let undetermined = dynamic_width.is_none() && static_width.is_none();
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
    let total_bw = if let Some((_, bw)) = static_width {
        Some(bw)
    } else {
        None
    };

    // code gen
    let (constants, constant_ids) = code_gen_constants(&concats[0], total_bw);
    let precheck = String::new();
    let construct = String::new();
    let fielding = String::new();
    let returning = if return_source {
        RETURN_AWI.to_owned()
    } else {
        String::new()
    };
    if return_source {
        if let Some((_, _)) = static_width {
            // statically known bitwidth
        } else if let Some(_) = dynamic_width {
            // dynamically known bitwidth
            todo!()
        } else {
            // common alignment unbounded stuff
            todo!()
        }
    }

    // The output is wrapped in brackets
    Ok(format!(
        "{{\n{}{}{}{}{}\n}}",
        constants, precheck, construct, fielding, returning
    ))
}

fn code_gen_constant(awi: &Bits, id: usize) -> String {
    format!(
        "let {}_dynamic_constant_{} InlAwi::<{}, {}>::unstable_from_slice(&{:?})",
        PREFIX,
        id,
        awi.bw(),
        awi.raw_len(),
        awi.as_slice(),
    )
}

fn code_gen_constants(
    source: &Concatenation,
    total_bw: Option<NonZeroUsize>,
) -> (String, Vec<Option<usize>>) {
    let mut constants = String::new();
    let mut lsb_i = 0;
    let mut shl = 0;
    let mut msb_i = source.concatenation.len() - 1;
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
        // start again from the msb
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
        constants.push_str(&format!(
            "let {} = InlAwi::<{}, {}>::unstable_from_slice(&{:?});\n",
            RETURN_AWI,
            awi.bw(),
            awi.raw_len(),
            awi[..].as_raw_slice(),
        ));
    } else {
        constants.push_str(&format!(
            "let {} = ExtAwi::zero({});\n",
            RETURN_AWI, DYNAMIC_BW
        ));
    }
    // The following code is quite ugly, but trying to do this all in one loop is
    // insanely error prone

    // Now add constants that "float" in the middle because of dynamic ranges.
    // Allocate consecutive static ranges together.
    let mut constant_ids = vec![None; source.concatenation.len()];
    let mut id_num = 0;
    let mut consecutive = false;
    for i in lsb_i..msb_i {
        let comp = &source.concatenation[i];
        if comp.range.static_range().is_some() {
            if !consecutive {
                if matches!(comp.component_type, Literal(_)) {
                    consecutive = true;
                }
            }
            constant_ids[i] = Some(id_num);
        } else if consecutive {
            consecutive = false;
            id_num += 1;
        }
    }
    // trim the ends of the allocations to not include non-literals
    let mut prev_id = None;
    let mut trim = false;
    for i in (lsb_i..msb_i).rev() {
        let comp = &source.concatenation[i];
        if let Some(id) = constant_ids[i] {
            if Some(id) != prev_id {
                prev_id = Some(id);
                trim = true;
            }
            if trim {
                if matches!(comp.component_type, Literal(_)) {
                    trim = false;
                } else {
                    constant_ids[i] = None;
                }
            }
        }
    }
    // calculate widths for every allocation
    let mut widths = Vec::new();
    for i in lsb_i..msb_i {
        if constant_ids[i].is_some() {
            let comp = &source.concatenation[i];
            // the ids start at zero, so `widths` can later be indexed by id
            widths.push(comp.range.static_width().unwrap());
        }
    }
    // create the constants
    let mut awi: Option<ExtAwi> = None;
    let mut shl = 0;
    let mut prev_id = None;
    for i in lsb_i..msb_i {
        let comp = &source.concatenation[i];
        if let Some(id) = constant_ids[i] {
            if Some(id) != prev_id {
                // reset for the next constant
                if let Some(awi) = awi {
                    constants.push_str(&code_gen_constant(&awi[..], prev_id.unwrap()))
                }
                awi = Some(ExtAwi::zero(bw(widths[id])));
                shl = 0;
                prev_id = Some(id);
            }
            let r = comp.range.static_range().unwrap();
            let w = comp.range.static_width().unwrap();
            match comp.component_type {
                Literal(ref lit) => {
                    if let Some(ref mut awi) = awi {
                        awi[..].field(shl, &lit[..], r.0, w).unwrap();
                    } else {
                        unreachable!()
                    }
                }
                Variable(_) => (),
                Filler => unreachable!(),
            }
            shl += w;
        }
    }
    // push last constant
    if let Some(awi) = awi {
        constants.push_str(&code_gen_constant(&awi[..], prev_id.unwrap()))
    }
    (constants, constant_ids)
}

/*
#[test]
fn code_gen_test() {
}
*/
