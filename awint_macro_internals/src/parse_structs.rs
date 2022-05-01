use awint_core::Bits;

use crate::{chars_to_string, usb_common_case};

pub fn usize_to_i128(x: usize) -> Result<i128, String> {
    i128::try_from(x).map_err(|_| "i128::try_from overflow".to_owned())
}

/// Usize and/or String Bound. If `s.is_empty()`, then there is no arbitrary
/// string in the bound and the base value is 0. `x` is added on to the value.
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Usb {
    pub s: Vec<char>,
    pub x: i128,
}

impl Usb {
    pub const fn zero() -> Self {
        Self { s: vec![], x: 0 }
    }

    pub fn new(s: &[char], x: i128) -> Self {
        Usb { s: s.to_owned(), x }
    }

    pub fn new_s(s: &[char]) -> Self {
        Usb {
            s: s.to_owned(),
            x: 0,
        }
    }

    pub const fn val(x: i128) -> Self {
        Self { s: vec![], x }
    }

    /// Tries to parse the `s` part of `self` as an integer and adds it to `x`.
    pub fn simplify(&mut self) -> Result<(), String> {
        if !self.s.is_empty() {
            if let Ok(x) = chars_to_string(&self.s).parse::<i128>() {
                self.s.clear();
                self.x = self.x.checked_add(x).unwrap();
            } else {
                *self = usb_common_case(&self.s)?;
            }
        }
        // we could determine that value is negative, but for better error reporting I
        // want it at the range level
        /*if self.s.is_empty() {
            if self.x < 0 {
                return Err("determined statically that this is negative")
            }
        }*/
        Ok(())
    }

    pub fn static_val(&self) -> Option<i128> {
        if self.s.is_empty() {
            Some(self.x)
        } else {
            None
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
            chars_to_string(&self.s)
        } else {
            format!("({} + {})", chars_to_string(&self.s), self.x)
        }
    }
}

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct Usbr {
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

    pub fn single_bit(s: &[char]) -> Self {
        Self {
            start: Some(Usb::new(s, 0)),
            end: Some(Usb::new(s, 1)),
        }
    }

    pub fn new_static(start: i128, end: i128) -> Self {
        Usbr {
            start: Some(Usb::val(start)),
            end: Some(Usb::val(end)),
        }
    }

    /// Also tries to statically check if range is valid
    pub fn simplify(&mut self) -> Result<(), String> {
        if let Some(ref mut start) = self.start {
            start.simplify()?;
        } else {
            self.start = Some(Usb::zero());
        }
        if let Some(ref mut end) = self.end {
            end.simplify()?;
        }
        // note: we cannot simplify `(same + x)..(same + y)` any because we still have
        // to perform checks about the range sides being in bounds, but we can still
        // calculate static ranges.

        if let Some(ref mut start) = self.start {
            if let Some(val) = start.static_val() {
                if val < 0 {
                    // make it generic because the simplification can move things around
                    return Err(
                        "determined statically that this range has a negative bound".to_owned()
                    )
                }
            }
        }
        if let Some(ref mut end) = self.end {
            if let Some(val) = end.static_val() {
                if val < 0 {
                    return Err(
                        "determined statically that this range has a negative bound".to_owned()
                    )
                }
            }
        }
        if let Some((r0, r1)) = self.static_range() {
            if r0 > r1 {
                return Err("determined statically that this is a reversed range".to_owned())
            }
            if r0 == r1 {
                // I *think* this actually might be required in some usages of the range, and is
                // not just a warning
                return Err("determined statically that this range has zero bitwidth".to_owned())
            }
        }
        Ok(())
    }

    /// Attempt to simplify the range for literal components. Returns an error
    /// if the function statically finds the range to be out of
    /// bounds of `bits`
    ///
    /// Assumes `simplify` has already been called
    pub fn simplify_literal(&mut self, bits: &Bits) -> Result<(), String> {
        if let Some(ref start) = self.start {
            if let Some(x) = start.static_val() {
                if x >= usize_to_i128(bits.bw())? {
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
                if x > usize_to_i128(bits.bw())? {
                    return Err(format!(
                        "end of range ({}) statically determined to be greater than the bitwidth \
                         of the literal ({})",
                        x,
                        bits.bw()
                    ))
                }
            }
        } else {
            self.end = Some(Usb::val(usize_to_i128(bits.bw())?));
        }
        Ok(())
    }

    /// Returns if a static range was able to be determined
    pub fn static_range(&self) -> Option<(i128, i128)> {
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

    /// Returns if a static width was able to be determined
    pub fn static_width(&self) -> Option<i128> {
        if let Some(ref start) = self.start {
            if let Some(ref end) = self.end {
                if start.s == end.s {
                    return end.x.checked_sub(start.x)
                }
            }
        }
        None
    }

    /// Returns an error if ranges are statically determined to be invalid
    pub fn simplify_filler(&mut self) -> Result<(), String> {
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
}
