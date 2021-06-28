//! for lowering of individual structs

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};

use awint_ext::ExtAwi;

use crate::*;

/// Exists for ordering `ExtAwi`s first by bitwidth, then by unsigned value
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub(crate) struct OrdExtAwi(pub ExtAwi);

#[allow(clippy::comparison_chain)]
impl Ord for OrdExtAwi {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.0.bw() < other.0.bw() {
            Ordering::Less
        } else if self.0.bw() == other.0.bw() {
            if self.0.const_as_ref().ult(other.0.const_as_ref()).unwrap() {
                Ordering::Less
            } else if self == other {
                Ordering::Equal
            } else {
                Ordering::Greater
            }
        } else {
            Ordering::Greater
        }
    }
}

impl PartialOrd for OrdExtAwi {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Returns a name for `self`. `literal_to_id` should be set if this component
/// could be a literal. Returns `None` if `self` is a filler.
pub(crate) fn lowered_name(
    literal_to_id: Option<&HashMap<ExtAwi, usize>>,
    comp: &Component,
) -> Option<String> {
    match comp.component_type {
        Literal(ref lit) => Some(format!("{}_{}", CONSTANT, literal_to_id.unwrap()[lit])),
        Variable(ref var) => Some(var.clone()),
        Filler => None,
    }
}

/// Collect all values that the component checks, concat checks, or fielding may
/// need
pub(crate) fn lower_values(
    values: &mut HashSet<String>,
    literal_to_id: &HashMap<ExtAwi, usize>,
    comp: &Component,
) {
    if let Some(w) = comp.range.static_width() {
        values.insert(format!("{}", w));
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
    if let Some(name) = lowered_name(Some(literal_to_id), comp) {
        values.insert(name + ".bw()");
    }
    if let Some(start) = start {
        values.insert(start.lowered_value());
    }
    if let Some(end) = end {
        values.insert(end.lowered_value());
    }
}

/// Collect all component range checks
pub(crate) fn lower_component_checks(
    lt_checks: &mut HashSet<(String, String)>,
    literal_to_id: &HashMap<ExtAwi, usize>,
    comp: &Component,
) {
    // Add as few checks as we statically determine possible
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
                lt_checks.insert((end.lowered_value(), start.lowered_value()));
            }
        }
    } else if let Some(name) = lowered_name(Some(literal_to_id), comp) {
        match (start, end) {
            (Some(start), Some(end)) => {
                // x.bw() < end
                lt_checks.insert((name + ".bw()", end.lowered_value()));
                // end < start
                if let (Some(start), Some(end)) = (start.static_val(), end.static_val()) {
                    assert!(start <= end);
                } else {
                    lt_checks.insert((end.lowered_value(), start.lowered_value()));
                }
            }
            (Some(start), None) => {
                // x.bw() < start
                lt_checks.insert((name + ".bw()", start.lowered_value()));
            }
            (None, Some(end)) => {
                // x.bw() < end
                lt_checks.insert((name + ".bw()", end.lowered_value()));
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

pub(crate) fn lower_width(
    literal_to_id: Option<&HashMap<ExtAwi, usize>>,
    comp: &Component,
) -> Option<Width> {
    if let Some(w) = comp.range.static_width() {
        Some(Width::Single(format!("{}", w)))
    } else {
        let name = lowered_name(literal_to_id, comp);
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
                unreachable!() // TODO I think the static checker doesn't
                               // catch this
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
    if partials.is_empty() {
        return "[]".to_owned()
    }
    let mut s = "[".to_owned();
    for i in 0..(partials.len() - 1) {
        s += &partials[i];
        s += ", ";
    }
    s += &partials[partials.len() - 1];
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
