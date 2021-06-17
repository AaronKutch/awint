//! This crate exists because of limitations with `proc-macro` crates. We need
//! to be able to test errors returned from the code generation function while
//! also being able to test the macros themselves.

// FIXME publish crate

#![feature(binary_heap_into_iter_sorted)]
#![allow(clippy::needless_range_loop)]
// TODO need a refactor
#![allow(clippy::too_many_arguments)]

mod lower_fields;
mod lower_structs;
mod lowering;
mod parse;
mod parse_structs;

use std::num::NonZeroUsize;

pub(crate) use lower_fields::*;
pub(crate) use lower_structs::*;
pub(crate) use lowering::*;
pub(crate) use parse::*;
pub(crate) use parse_structs::*;

// TODO prefix "__awint_" on all these

/// Prefix used for constants
pub(crate) const CONSTANT: &str = "__constant";
/// Prefix used for values
pub(crate) const VALUE: &str = "__val";
/// Prefix used for widths
pub(crate) const WIDTH: &str = "__width";
/// Prefix used for bitwidths
pub(crate) const BW: &str = "__bw";
/// Prefix used for `Bits` references
pub(crate) const REF: &str = "__ref";
/// Name used by the construct which might be created for returning, created as
/// a temporary only, or never created.
pub(crate) const AWI: &str = "__awi";
/// Name used for the reference to `AWI`
pub(crate) const AWI_REF: &str = "__awi_ref";
/// Name used for the fielding `to` offset
pub(crate) const SHL: &str = "__shl";

use ComponentType::*;

/// Code generation for concatenations of components macros. `input` is the
/// string input to the macro. `specified_source` is true if source must be
/// specified and there can be no fillers in the source. `construct_fn` is the
/// construction function used (e.x. "zero", "umax", etc, "zero" should be used
/// by default). `inlawi` is if an `InlAwi` should be the return type.
/// `return_souce` is if the value of the source should be returned.
///
/// # Errors
///
/// If one of the many error conditions described by the `awint_macros`
/// documentation occurs, then this will return a string representation of that
/// error.
pub fn code_gen(
    input: &str,
    specified_source: bool,
    construct_fn: &str,
    inlawi: bool,
    return_source: bool,
) -> Result<String, String> {
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
                if (j0 == 0) && !specified_source && return_source {
                    return Err(
                        "a construction macro with unspecified initialization cannot have a \
                         filler in the source concatenation"
                            .to_owned(),
                    )
                }
                if component.range.end.is_none() {
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

    Ok(lower(
        &concats,
        dynamic_width_i,
        total_bw,
        specified_source,
        undetermined,
        construct_fn,
        inlawi,
        return_source,
    ))
}
