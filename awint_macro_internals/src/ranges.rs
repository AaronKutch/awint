use awint_core::Bits;
use triple_arena::Ptr;

use crate::{chars_to_string, usize_to_i128, Ast, CCMacroError, Delimiter, PText, Text};

/// Tries parsing as hexadecimal, octal, binary, and decimal
pub fn i128_try_parse(s: &[char]) -> Option<i128> {
    let mut s = s;
    if s.is_empty() {
        return None
    }
    let mut neg = false;
    if s[0] == '-' {
        neg = true;
        s = &s[1..];
        if s.is_empty() {
            return None
        }
    }
    let val = if (s.len() > 2) && (s[0] == '0') {
        if s[1] == 'x' {
            i128::from_str_radix(&chars_to_string(&s[2..]), 16).ok()
        } else if s[1] == 'o' {
            i128::from_str_radix(&chars_to_string(&s[2..]), 8).ok()
        } else if s[1] == 'b' {
            i128::from_str_radix(&chars_to_string(&s[2..]), 2).ok()
        } else {
            None
        }
    } else {
        chars_to_string(s).parse().ok()
    };
    if let Some(val) = val {
        if neg {
            val.checked_neg()
        } else {
            Some(val)
        }
    } else {
        None
    }
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
    /// Performs advanced simplifications such as interpreting
    /// `({+/-}{string/i128} {+/-} {+/-}{string/i128})`.
    /// Returns `true` if simplification happened
    pub fn simplify(&mut self) -> Result<(), String> {
        if !self.s.is_empty() {
            if let Some(x) = i128_try_parse(&self.s) {
                self.s.clear();
                self.x = self
                    .x
                    .checked_add(x)
                    .ok_or_else(|| "i128 overflow".to_owned())?;
            }
        }
        // note: we could determine now if the value is negative, but for better
        // error reporting I want it at the range level
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
                        "determined statically that this has a range with a negative bound"
                            .to_owned(),
                    )
                }
            }
        }
        if let Some(ref mut end) = self.end {
            if let Some(val) = end.static_val() {
                if val < 0 {
                    return Err(
                        "determined statically that this has a range with a negative bound"
                            .to_owned(),
                    )
                }
            }
        }
        if let Some((r0, r1)) = self.static_range() {
            if r0 > r1 {
                return Err("determined statically that this has a reversed range".to_owned())
            }
            if r0 == r1 {
                // this is required for literals that would take up a concatenation
                return Err("determined statically that this has a zero bitwidth range".to_owned())
            }
        }
        // `static_width` does the equal string check
        if let Some(w) = self.static_width() {
            if w < 0 {
                return Err("determined statically that this has a reversed range".to_owned())
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
        let bits_bw = usize_to_i128(bits.bw())?;
        if let Some(ref start) = self.start {
            if let Some(x) = start.static_val() {
                if x >= bits_bw {
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
                if x > bits_bw {
                    return Err(format!(
                        "end of range ({}) statically determined to be greater than the bitwidth \
                         of the literal ({})",
                        x,
                        bits.bw()
                    ))
                }
            }
        } else {
            self.end = Some(Usb::val(bits_bw));
        }
        if let Some(w) = self.static_width() {
            if w > bits_bw {
                return Err(format!(
                    "width of range ({}) statically determined to be greater than the bitwidth of \
                     the literal ({})",
                    w,
                    bits.bw()
                ))
            }
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
                    "a filler with a bounded start should also have a bounded end".to_owned(),
                )
            }
        }
        Ok(())
    }
}

/// In ranges we commonly see stuff like `(x + y)` or `(x - y)` with one of them
/// being a constant we can parse, which passes upward the `Usb` and `Usbr`
/// chain to get calculated into a static width.
pub fn parse_usb(ast: &mut Ast, usb_txt: Ptr<PText>) -> Result<Usb, CCMacroError> {
    assert!(!ast.txt[usb_txt].is_empty());
    let mut usb_inner = usb_txt;
    let mut invalid = false;
    if ast.txt[usb_txt].len() != 1 {
        invalid = true;
    }
    if let Text::Group(d, p) = ast.txt[usb_txt][0] {
        usb_inner = p;
        if d != Delimiter::Parenthesis {
            invalid = true;
        }
    }
    if invalid {
        // prevents the syntax of cc ranges from diverging from normal ranges, prevents
        // confusion
        let mut s = vec![];
        ast.chars_assign_subtree(&mut s, usb_txt);
        return Err(CCMacroError {
            error: "bounds of ranges need to be a single Rust literal, identifier, or parenthesis \
                    delimited group"
                .to_owned(),
            red_text: vec![usb_txt],
            help: Some(format!(
                "wrap the bound in parenthesis like `({})`",
                chars_to_string(&s)
            )),
        })
    }

    let mut seen_plus = Vec::<usize>::new();
    let mut seen_minus = Vec::<usize>::new();
    let mut string = vec![];
    let usb_len = ast.txt[usb_inner].len();
    for i in 0..usb_len {
        match ast.txt[usb_inner][i] {
            crate::Text::Chars(ref s) => {
                if s.len() == 1 {
                    // must be punctuation
                    let c = s[0];
                    if c == '+' {
                        seen_plus.push(string.len());
                    } else if c == '-' {
                        seen_minus.push(string.len());
                    }
                    string.push(c);
                } else {
                    string.extend(s);
                }
            }
            crate::Text::Group(ref d, ref p) => {
                string.extend(d.lhs_chars());
                ast.chars_assign_subtree(&mut string, *p);
                string.extend(d.rhs_chars());
            }
        }
    }
    // fallback
    let original = Usb::new_s(&string);
    let mut lhs_rhs = None;
    let mut neg = false;
    if !seen_plus.is_empty() {
        lhs_rhs = Some((
            Usb::new_s(&string[..*seen_plus.last().unwrap()]),
            Usb::new_s(&string[(*seen_plus.last().unwrap() + 1)..]),
        ));
    } else if !seen_minus.is_empty() {
        let mut mid = None;
        // search for rightmost adjacent '-'s, e.x. (a - -8) which got compressed
        for i in (0..(seen_minus.len() - 1)).rev() {
            if (seen_minus[i] + 1) == seen_minus[i + 1] {
                mid = Some(seen_minus[i]);
            }
        }
        // else just use last minus
        if mid.is_none() {
            mid = Some(*seen_minus.last().unwrap());
        }
        if let Some(mid) = mid {
            lhs_rhs = Some((Usb::new_s(&string[..mid]), Usb::new_s(&string[(mid + 1)..])));
            neg = true;
        }
    }
    if let Some((mut lhs, mut rhs)) = lhs_rhs {
        lhs.simplify().map_err(|e| {
            CCMacroError::new(
                format!("failed simplifying left side of subexpression: {}", e),
                usb_txt,
            )
        })?;
        rhs.simplify().map_err(|e| {
            CCMacroError::new(
                format!("failed simplifying right side of subexpression: {}", e),
                usb_txt,
            )
        })?;
        let overflow = || CCMacroError::new("i128 overflow".to_owned(), usb_txt);
        if let Some(rhs) = rhs.static_val() {
            if neg {
                lhs.x = lhs.x.checked_sub(rhs).ok_or_else(overflow)?;
            } else {
                lhs.x = lhs.x.checked_add(rhs).ok_or_else(overflow)?;
            }
            Ok(lhs)
        } else if let Some(lhs) = lhs.static_val() {
            rhs.x = rhs.x.checked_add(lhs).ok_or_else(overflow)?;
            if neg {
                // compiler will handle the '-' later
                rhs.s.insert(0, '-');
            }
            Ok(rhs)
        } else {
            Ok(original)
        }
    } else {
        Ok(original)
    }
}

/// Tries to parse raw `input` as a range. Looks for the existence of top
/// level ".." or "..=" punctuation. If `allow_single_bit_range` is set, will
/// return a single bit range if ".." or "..=" does not exist.
pub fn parse_range(
    ast: &mut Ast,
    range_txt: Ptr<PText>,
    allow_single_bit_range: bool,
) -> Result<Usbr, Option<CCMacroError>> {
    // We want to do the ".."/"..=" separation followed by "+"/"-" optimization, so
    // we need to preserve group boundaries one more time.

    if ast.txt[range_txt].is_empty() {
        return Err(Some(CCMacroError::new(
            "range is empty".to_owned(),
            range_txt,
        )))
    }

    // inclusive index of the first and exclusive index of the last char
    let mut range = None;
    let mut inclusive = false;
    let range_len = ast.txt[range_txt].len();
    let mut dots = 0;
    let double_err = || {
        Err(Some(CCMacroError::new(
            "encountered two top level \"..\" strings in same range".to_owned(),
            range_txt,
        )))
    };
    for i in 0..range_len {
        let next_dots;
        match ast.txt[range_txt][i] {
            Text::Chars(ref s) => {
                if s.len() == 1 {
                    // must be punctuation
                    let c = s[0];
                    if c == '=' {
                        if dots == 2 {
                            // inclusive range
                            inclusive = true;
                            if range.is_some() {
                                return double_err()
                            }
                            range = Some((i - 2, i + 1));
                            dots = 0;
                        }
                        next_dots = 0;
                    } else if c == '.' {
                        next_dots = dots + 1;
                        if next_dots == 3 {
                            return Err(Some(CCMacroError::new(
                                "encountered top level deprecated \"...\" string in range"
                                    .to_owned(),
                                range_txt,
                            )))
                        }
                    } else {
                        next_dots = 0;
                    }
                } else {
                    next_dots = 0;
                }
            }
            Text::Group(..) => next_dots = 0,
        }
        if next_dots == 0 && dots == 2 {
            // exclusive range
            if range.is_some() {
                return double_err()
            }
            range = Some((i - 2, i));
            dots = 0;
        } else {
            dots = next_dots;
        }
    }
    if dots == 2 {
        if range.is_some() {
            return double_err()
        }
        // the range ended with ".."
        range = Some((range_len - 2, range_len));
    }
    if let Some((lo, hi)) = range {
        // Subdivide `range_txt`. note if we did the pop strategy we would also have to
        // `reverse` the arrays.
        let lhs_txt: Vec<Text> = ast.txt[range_txt][..lo].to_vec();
        let mid_txt: Vec<Text> = ast.txt[range_txt][lo..hi].to_vec();
        let rhs_txt: Vec<Text> = ast.txt[range_txt][hi..].to_vec();
        ast.txt[range_txt].clear();
        let lhs_empty = lhs_txt.is_empty();
        let rhs_empty = rhs_txt.is_empty();
        let rhs_p = ast.txt.insert(rhs_txt);
        let mid_p = ast.txt.insert(mid_txt);
        let lhs_p = ast.txt.insert(lhs_txt);
        ast.txt[range_txt].push(Text::Group(Delimiter::None, lhs_p));
        ast.txt[range_txt].push(Text::Group(Delimiter::None, mid_p));
        ast.txt[range_txt].push(Text::Group(Delimiter::None, rhs_p));
        let start = if lhs_empty {
            Usb::zero()
        } else {
            parse_usb(ast, lhs_p)?
        };
        let mut end = if rhs_empty {
            None
        } else {
            Some(parse_usb(ast, rhs_p)?)
        };
        if inclusive {
            if let Some(ref mut end) = end {
                end.x = end
                    .x
                    .checked_add(1)
                    .ok_or_else(|| CCMacroError::new("i128 overflow".to_owned(), rhs_p))?;
            }
        }
        Ok(Usbr {
            start: Some(start),
            end,
        })
    } else if allow_single_bit_range {
        // single bit range
        let mut start = Usb::zero();
        ast.chars_assign_subtree(&mut start.s, range_txt);
        let mut end = start.clone();
        end.x = 1;
        Ok(Usbr {
            start: Some(start),
            end: Some(end),
        })
    } else {
        // don't put anything, this error is dropped
        Err(None)
    }
}
