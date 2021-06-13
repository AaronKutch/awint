use alloc::{borrow::ToOwned, format, string::String, vec::Vec};
use core::{convert::TryInto, num::NonZeroUsize};

use awint_core::Bits;

use crate::ExtAwi;

/// Name used by the return construct
pub(crate) const RET_AWI: &str = "__ret_awi";
/// Prefix used for constants
pub(crate) const CONSTANT: &str = "__constant";
/// Name used for the common concatenation bitwidth
pub(crate) const BW: &str = "__bw";
/// Name used for the field `to` offset
pub(crate) const SHL: &str = "__shl";

/// Usize and/or String Bound. If `s.is_empty()`, then there is no arbitrary
/// string in the bound and the base value is 0. `x` is added on to the value.
#[derive(Debug, Clone)]
pub(crate) struct Usb {
    pub s: String,
    pub x: i128,
}

impl Usb {
    pub const fn zero() -> Self {
        Self {
            s: String::new(),
            x: 0,
        }
    }

    pub fn new(s: &str, x: i128) -> Self {
        Usb { s: s.to_owned(), x }
    }

    pub const fn val(x: i128) -> Self {
        Self {
            s: String::new(),
            x,
        }
    }

    pub fn attempt_constify(&mut self) {
        if !self.s.is_empty() {
            if let Ok(x) = self.s.parse::<i128>() {
                self.s.clear();
                self.x = self.x.checked_add(x).unwrap();
            }
        }
    }

    pub fn static_val_i128(&self) -> Option<i128> {
        if self.s.is_empty() {
            Some(self.x)
        } else {
            None
        }
    }

    pub fn static_val(&self) -> Option<usize> {
        if self.s.is_empty() {
            self.x.try_into().ok()
        } else {
            None
        }
    }

    pub fn check_fits_usize(&self) -> Result<(), String> {
        if self.static_val_i128().is_some() && self.static_val().is_none() {
            Err("static bound is negative or does not fit in a `usize`".to_owned())
        } else {
            Ok(())
        }
    }

    /// Returns if we statically determine the bitwidth to be zero
    pub fn is_guaranteed_zero(&self) -> bool {
        if self.s.is_empty() {
            self.x == 0
        } else {
            false
        }
    }

    pub fn code_gen_value(&self) -> String {
        if self.s.is_empty() {
            format!("{}", self.x)
        } else if self.x == 0 {
            format!("{}", self.s)
        } else {
            format!("({} + {})", self.s, self.x)
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Usbr {
    pub start: Option<Usb>,
    pub end: Option<Usb>,
}

/// A range encompassing `self.start..self.end`
impl Usbr {
    pub fn unbounded() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    pub fn single_bit(s: &str) -> Self {
        Self {
            start: Some(Usb {
                s: s.to_owned(),
                x: 0,
            }),
            end: Some(Usb {
                s: s.to_owned(),
                x: 1,
            }),
        }
    }

    pub fn new_static(start: usize, end: usize) -> Self {
        Usbr {
            start: Some(Usb::val(start as i128)),
            end: Some(Usb::val(end as i128)),
        }
    }

    pub fn static_range(&self) -> Option<(usize, usize)> {
        if let Some(ref start) = self.start {
            if let Some(start) = start.static_val() {
                if let Some(ref end) = self.end {
                    if let Some(end) = end.static_val() {
                        return Some((start, end))
                    }
                }
            }
        }
        None
    }

    /// Panics if the range is reversed
    pub fn static_width(&self) -> Option<usize> {
        self.static_range().map(|r| r.1.checked_sub(r.0).unwrap())
    }

    pub fn attempt_constify_general(&mut self) {
        if let Some(ref mut start) = self.start {
            start.attempt_constify();
        }
        if let Some(ref mut end) = self.end {
            end.attempt_constify();
        }
    }

    /// Returns an error if the function statically finds the range to be out of
    /// bounds of `bits`
    pub fn attempt_constify_literal(&mut self, bits: &Bits) -> Result<(), String> {
        self.attempt_constify_general();
        if let Some(ref start) = self.start {
            if let Some(x) = start.static_val() {
                if x >= bits.bw() {
                    return Err(format!(
                        "start of range ({}) statically determined to be greater than or equal to \
                         the bitwidth of the literal ({})",
                        x,
                        bits.bw()
                    ))
                }
            }
        } else {
            self.start = Some(Usb::zero());
        }
        if let Some(ref end) = self.end {
            if let Some(x) = end.static_val() {
                if x > bits.bw() {
                    return Err(format!(
                        "end of range ({}) statically determined to be greater than the bitwidth \
                         of the literal ({})",
                        x,
                        bits.bw()
                    ))
                }
            }
        } else {
            self.end = Some(Usb::val(bits.bw().try_into().unwrap()));
        }
        self.check_fits_usize_range()
    }

    pub fn attempt_constify_filler_and_variable(&mut self) -> Result<(), String> {
        self.attempt_constify_general();
        if self.start.is_none() {
            self.start = Some(Usb::zero());
        }
        self.check_fits_usize_range()
    }

    pub fn check_fits_usize_range(&self) -> Result<(), String> {
        if let Some(ref start) = self.start {
            start.check_fits_usize()?;
        }
        if let Some(ref end) = self.end {
            end.check_fits_usize()?;
        }
        if let Some((r0, r1)) = self.static_range() {
            if r0 > r1 {
                return Err("has a reversed range".to_owned())
            }
            if r0 == r1 {
                return Err("determined statically that this has zero bitwidth".to_owned())
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ComponentType {
    Literal(ExtAwi),
    Variable(String),
    Filler,
}

use ComponentType::*;

#[derive(Debug, Clone)]
pub(crate) struct Component {
    pub component_type: ComponentType,
    pub range: Usbr,
}

impl Component {
    pub fn new(component_type: ComponentType, range: Usbr) -> Self {
        Self {
            component_type,
            range,
        }
    }

    pub fn is_static_literal(&self) -> bool {
        if let Literal(_) = self.component_type {
            self.range.static_range().is_some()
        } else {
            false
        }
    }

    /// An error is returned only if some statically determined bound is
    /// violated, not if the constify attempt fails
    pub fn attempt_constify(&mut self) -> Result<(), String> {
        if let Literal(ref mut lit) = self.component_type.clone() {
            // note: `lit` here is a reference to a clone
            self.range.attempt_constify_literal(lit.const_as_ref())?;
            // static ranges on literals have been verified, attempt to truncate
            if let Some(x) = self.range.end.clone().map(|x| x.static_val()).flatten() {
                let mut tmp = ExtAwi::zero(NonZeroUsize::new(x).unwrap());
                tmp[..].zero_resize_assign(&lit[..]);
                *lit = tmp;
            }
            if let Some(x) = self.range.start.clone().map(|x| x.static_val()).flatten() {
                let w = lit.bw() - x;
                let mut tmp = ExtAwi::zero(NonZeroUsize::new(w).unwrap());
                tmp[..].field(0, &lit[..], x, w);
                *lit = tmp;
                self.range.start.as_mut().unwrap().x = 0;
                self.range.end.as_mut().unwrap().x = w as i128;
            }
            self.component_type = Literal(lit.clone());
        } else {
            self.range.attempt_constify_filler_and_variable()?;
        }
        Ok(())
    }

    /// Panics if a name cannot be found for `self`. `lit_id` is the id for a
    /// literal.
    pub fn code_gen_name(&self, lit_id: Option<usize>) -> String {
        match self.component_type {
            Literal(_) => format!("{}_{}", CONSTANT, lit_id.unwrap()),
            Variable(ref var) => var.clone(),
            Filler => unreachable!(),
        }
    }

    /// The name associated with a potential literal is `literal_name`. This is
    /// ignored if the component has all static bounds or is not a literal.
    pub fn code_gen_bw(&self, literal_name: Option<String>) -> Option<String> {
        if let Some(w) = self.range.static_width() {
            Some(format!("{}", w))
        } else {
            let name = match self.component_type {
                Literal(_) => literal_name,
                Variable(ref var) => Some(var.clone()),
                Filler => None,
            };
            // simple 0 optimization
            let start = if let Some(ref start) = self.range.start {
                if start.is_guaranteed_zero() {
                    None
                } else {
                    Some(start.clone())
                }
            } else {
                None
            };
            let end = if let Some(ref end) = self.range.end {
                if end.is_guaranteed_zero() {
                    unreachable!() // TODO I think the static checker doesn't
                                   // catch this
                } else {
                    Some(end.clone())
                }
            } else {
                None
            };
            match (start, end) {
                (Some(start), Some(end)) => Some(format!(
                    "({} - {})",
                    end.code_gen_value(),
                    start.code_gen_value()
                )),
                (Some(start), None) => {
                    name.map(|name| format!("({}.bw() - {})", name, start.code_gen_value()))
                }
                (None, Some(end)) => {
                    name.map(|name| format!("({} - {}.bw())", end.code_gen_value(), name))
                }
                (None, None) => name.map(|name| format!("{}.bw()", name)),
            }
        }
    }

    // start >= x.bw() || end > x.bw() || start > end
    pub fn code_gen_bounds_check(
        &self,
        literal_name: Option<String>,
    ) -> Result<Option<String>, ()> {
        if let Filler = self.component_type {
            match (self.range.start.clone(), self.range.end.clone()) {
                (Some(start), Some(end)) => Ok(Some(format!(
                    "({} > {})",
                    start.code_gen_value(),
                    end.code_gen_value()
                ))),
                _ => Ok(None),
            }
        } else {
            let name = match self.component_type {
                Literal(_) => literal_name.unwrap(),
                Variable(ref var) => var.clone(),
                Filler => unreachable!(),
            };
            // simple 0 optimization
            let start = if let Some(ref start) = self.range.start {
                if start.is_guaranteed_zero() {
                    None
                } else {
                    Some(start.clone())
                }
            } else {
                None
            };
            let end = if let Some(ref end) = self.range.end {
                if end.is_guaranteed_zero() {
                    unreachable!() // TODO I think the static checker doesn't
                                   // catch this
                } else {
                    Some(end.clone())
                }
            } else {
                None
            };
            match (start, end) {
                (Some(start), Some(end)) => Ok(Some(format!(
                    "({} >= {}.bw()) || ({} > {}.bw()) || ({} > {})",
                    start.code_gen_value(),
                    name,
                    end.code_gen_value(),
                    name,
                    start.code_gen_value(),
                    end.code_gen_value(),
                ))),
                (Some(start), None) => Ok(Some(format!(
                    "({} >= {}.bw())",
                    start.code_gen_value(),
                    name,
                ))),
                (None, Some(end)) => {
                    Ok(Some(format!("({} > {}.bw())", end.code_gen_value(), name,)))
                }
                _ => Ok(None),
            }
        }
    }
}

pub(crate) struct Concatenation {
    pub concatenation: Vec<Component>,
    pub total_bw: Option<NonZeroUsize>,
}
