use std::{convert::TryInto, num::NonZeroUsize};

use awint_core::Bits;

/// Usize and/or String Bound. If `s.is_empty()`, then there is no arbitrary
/// string in the bound and the base value is 0. `u` is added on to the value.
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
        Ok(())
    }

    pub fn attempt_constify_filler_and_variable(&mut self) {
        self.attempt_constify_general();
        if self.start.is_none() {
            self.start = Some(Usb::zero());
        }
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

use awint_ext::ExtAwi;
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

    /// An error is returned only if some statically determined bound is
    /// violated, not if the constify attempt fails
    pub fn attempt_constify(&mut self) -> Result<(), String> {
        match self.component_type {
            Literal(ref lit) => {
                self.range.attempt_constify_literal(lit.const_as_ref())?;
            }
            _ => self.range.attempt_constify_filler_and_variable(),
        }
        self.range.check_fits_usize_range()?;
        Ok(())
    }
}

pub(crate) struct Concatenation {
    pub concatenation: Vec<Component>,
    pub total_bw: Option<NonZeroUsize>,
}
