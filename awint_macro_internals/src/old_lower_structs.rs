//! for lowering of individual structs

use std::{collections::HashMap, hash::Hash};

use awint_ext::ExtAwi;
use triple_arena::Ptr;

use crate::*;

/// Returns a name for `self`. Returns `None` if `self` is a filler.
///
/// # Panics
///
/// If `literals` was not set and the component was a literal
pub(crate) fn lowered_name(
    literals: Option<&BiMap<Lit, ExtAwi>>,
    comp: &Component,
) -> Option<String> {
    match comp.component_type {
        Literal(ref lit) => Some(format!("{}_{}", CONSTANT, literals.unwrap().get_id(lit))),
        Variable(ref var) => Some(var.clone()),
        Filler => None,
    }
}

/// Collect all bindings that the values will need. Bindings do not call `bw()`
/// to avoid recalling and borrowing issues.
pub(crate) fn lower_bindings(
    bindings: &mut BiMap<Bind, String>,
    literals: &BiMap<Lit, ExtAwi>,
    comp: &Component,
) {
    if let Some(w) = comp.range.static_width() {
        bindings.insert(format!("{}", w));
    }
    if let Some(name) = lowered_name(Some(literals), comp) {
        bindings.insert(name);
    }
    // simple 0 optimization
    let start = if let Some(ref start) = comp.range.start {
        if start.is_guaranteed_zero() {
            None
        } else {
            Some(start.clone())
        }
    } else {
        None
    };
    let end = if let Some(ref end) = comp.range.end {
        if end.is_guaranteed_zero() {
            unreachable!()
        } else {
            Some(end.clone())
        }
    } else {
        None
    };
    if let Some(start) = start {
        bindings.insert(start.lowered_value());
    }
    if let Some(end) = end {
        bindings.insert(end.lowered_value());
    }
}

/// Collect all component range checks
pub(crate) fn lower_component_checks(
    lt_checks: &mut BiMap<LtCheck, (Ptr<Val>, Ptr<Val>)>,
    literals: &BiMap<Lit, ExtAwi>,
    string_to_value_ptr: &HashMap<String, Ptr<Val>>,
    comp: &Component,
) {
    // Eliminate as many checks as we statically determine possible
    if comp.has_full_range() {
        return
    }
    // simple 0 optimization
    let start = if let Some(ref start) = comp.range.start {
        if start.is_guaranteed_zero() {
            None
        } else {
            Some(start.clone())
        }
    } else {
        None
    };
    let end = if let Some(ref end) = comp.range.end {
        if end.is_guaranteed_zero() {
            unreachable!()
        } else {
            Some(end.clone())
        }
    } else {
        None
    };
    if matches!(&comp.component_type, Filler) {
        if let (Some(start), Some(end)) = (start, end) {
            if let (Some(start), Some(end)) = (start.static_val(), end.static_val()) {
                // no need for check, just make sure that earlier steps did not break
                assert!(start <= end);
            } else {
                lt_checks.insert((
                    string_to_value_ptr[&end.lowered_value()],
                    string_to_value_ptr[&start.lowered_value()],
                ));
            }
        }
    } else if let Some(name) = lowered_name(Some(literals), comp) {
        match (start, end) {
            (Some(start), Some(end)) => {
                // x.bw() < end
                lt_checks.insert((
                    string_to_value_ptr[&(name + ".bw()")],
                    string_to_value_ptr[&end.lowered_value()],
                ));
                // end < start
                if let (Some(start), Some(end)) = (start.static_val(), end.static_val()) {
                    assert!(start <= end);
                } else {
                    lt_checks.insert((
                        string_to_value_ptr[&end.lowered_value()],
                        string_to_value_ptr[&start.lowered_value()],
                    ));
                }
            }
            (Some(start), None) => {
                // x.bw() < start
                lt_checks.insert((
                    string_to_value_ptr[&(name + ".bw()")],
                    string_to_value_ptr[&start.lowered_value()],
                ));
            }
            (None, Some(end)) => {
                // x.bw() < end
                lt_checks.insert((
                    string_to_value_ptr[&(name + ".bw()")],
                    string_to_value_ptr[&end.lowered_value()],
                ));
            }
            _ => (),
        }
    }
}

/// Bitwidth as described by a single value or one value minus another
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Width {
    Single(String),
    Range(String, String),
}

/// Note: `lowering.rs` handles the translation back to values,
/// `string_to_value` is not needed
pub(crate) fn lower_width(
    literals: Option<&BiMap<Lit, ExtAwi>>,
    comp: &Component,
) -> Option<Width> {
    if let Some(w) = comp.range.static_width() {
        Some(Width::Single(format!("{}", w)))
    } else {
        let name = lowered_name(literals, comp);
        // simple 0 optimization
        let start = if let Some(ref start) = comp.range.start {
            if start.is_guaranteed_zero() {
                None
            } else {
                Some(start.clone())
            }
        } else {
            None
        };
        let end = if let Some(ref end) = comp.range.end {
            if end.is_guaranteed_zero() {
                unreachable!()
            } else {
                Some(end.clone())
            }
        } else {
            None
        };
        match (name, start, end) {
            // unbounded filler
            (None, None, None) => None,
            // literal or variable with full range
            (Some(name), None, None) => Some(Width::Single(format!("{}.bw()", name))),
            (Some(name), Some(start), None) => Some(Width::Range(
                start.lowered_value(),
                format!("{}.bw()", name),
            )),
            (_, Some(start), Some(end)) => {
                Some(Width::Range(start.lowered_value(), end.lowered_value()))
            }
            (_, None, Some(end)) => Some(Width::Single(end.lowered_value())),
            _ => unreachable!(),
        }
    }
}

/// Put the `partials` in a array like `[e0, e1, e2, ...]`
pub(crate) fn array_partials(partials: Vec<String>) -> String {
    let mut s = "[".to_owned();
    for i in 0..partials.len() {
        s += &partials[i];
        if i != (partials.len() - 1) {
            s += ", ";
        }
    }
    s += "]";
    s
}

pub(crate) fn add_partials(partials: Vec<String>) -> String {
    if partials.is_empty() {
        return String::new()
    }
    if partials.len() == 1 {
        return partials[0].clone()
    }
    let mut sum = String::new();
    for i in 0..(partials.len() - 1) {
        sum += &partials[i];
        sum += " + ";
    }
    sum += &partials[partials.len() - 1];
    sum
}
