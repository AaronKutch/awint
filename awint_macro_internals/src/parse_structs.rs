use std::{convert::TryInto, num::NonZeroUsize};

use awint_core::Bits;
use awint_ext::ExtAwi;

/// Usize and/or String Bound. If `s.is_empty()`, then there is no arbitrary
/// string in the bound and the base value is 0. `x` is added on to the value.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
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

    /// Tries to parse the `s` part of `self` as an integer and adds it to `x`.
    pub fn attempt_simplify(&mut self) {
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
            self.s.clone()
        } else {
            format!("({} + {})", self.s, self.x)
        }
    }

    pub fn lowered_value(&self) -> String {
        if self.s.is_empty() {
            format!("{}", self.x)
        } else if self.x == 0 {
            self.s.clone()
        } else {
            format!("({} + {})", self.s, self.x)
        }
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
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
            start: Some(Usb::new(s, 0)),
            end: Some(Usb::new(s, 1)),
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

    /// General purpose constification
    pub fn attempt_simplify_general(&mut self) {
        if let Some(ref mut start) = self.start {
            start.attempt_simplify();
        } else {
            self.start = Some(Usb::zero());
        }
        if let Some(ref mut end) = self.end {
            end.attempt_simplify();
        }
    }

    /// Attempt to simplify the range for literal components. Returns an error
    /// if the function statically finds the range to be out of
    /// bounds of `bits`
    pub fn attempt_simplify_literal(&mut self, bits: &Bits) -> Result<(), String> {
        self.attempt_simplify_general();
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

    /// Returns an error if ranges are statically determined to be invalid
    pub fn attempt_simplify_variable(&mut self) -> Result<(), String> {
        self.attempt_simplify_general();
        self.check_fits_usize_range()
    }

    /// Returns an error if ranges are statically determined to be invalid
    pub fn attempt_simplify_filler(&mut self) -> Result<(), String> {
        self.attempt_simplify_general();
        self.check_fits_usize_range()?;
        if let Some(ref start) = self.start {
            if !start.is_guaranteed_zero() && self.end.is_none() {
                // is a useless case anyway and prevents edge cases
                return Err(
                    "A filler with a bounded start should also have a bounded end".to_owned(),
                )
            }
        }
        Ok(())
    }

    /// Checks statically if `self` is a valid `usize` range. Returns an error
    /// if the range is reversed or has zero bitwidth.
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
        if let Literal(..) = self.component_type {
            self.range.static_range().is_some()
        } else {
            false
        }
    }

    /// Returns if the range on this component is "full". Unbounded fillers
    /// return false.
    pub fn has_full_range(&self) -> bool {
        if let Some(ref start) = self.range.start {
            if !start.is_guaranteed_zero() {
                return false
            }
        }
        match self.component_type {
            Literal(ref lit) => {
                if let Some(ref end) = self.range.end {
                    if !end.s.is_empty() || (end.x != (lit.bw() as i128)) {
                        return false
                    }
                }
                true
            }
            Variable(_) => self.range.end.is_none(),
            Filler => self.range.end.is_some(),
        }
    }

    /// An error is returned only if some statically determined bound is
    /// violated, not if the simplify attempt fails
    pub fn attempt_simplify(&mut self) -> Result<(), String> {
        match self.component_type.clone() {
            Literal(ref mut lit) => {
                // note: `lit` here is a reference to a clone
                self.range.attempt_simplify_literal(lit.const_as_ref())?;
                // static ranges on literals have been verified, attempt to truncate
                if let Some(x) = self.range.end.clone().and_then(|x| x.static_val()) {
                    let mut tmp = ExtAwi::zero(NonZeroUsize::new(x).unwrap());
                    tmp[..].zero_resize_assign(&lit[..]);
                    *lit = tmp;
                }
                // Note: I only truncate the start when the end is a plain static value, because
                // the subtraction from the end would introduce the possibility for an
                // underflow, and we would need yet another layer to the checks in the code gen.
                if let Some(ref end) = self.range.end {
                    if end.static_val().is_some() {
                        // attempt to truncate bits below the start
                        if let Some(x) = self.range.start.clone().and_then(|x| x.static_val()) {
                            let w = lit.bw() - x;
                            let mut tmp = ExtAwi::zero(NonZeroUsize::new(w).unwrap());
                            tmp[..].field(0, &lit[..], x, w);
                            *lit = tmp;
                            self.range.start.as_mut().unwrap().x = 0;
                            self.range.end.as_mut().unwrap().x -= x as i128;
                        }
                    }
                }
                self.component_type = Literal(lit.clone());
            }
            Variable(_) => self.range.attempt_simplify_variable()?,
            Filler => self.range.attempt_simplify_filler()?,
        }
        Ok(())
    }
}

pub(crate) struct Concatenation {
    pub concatenation: Vec<Component>,
    pub total_bw: Option<NonZeroUsize>,
}
