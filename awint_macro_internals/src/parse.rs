use std::{num::NonZeroUsize, str::FromStr};

use awint_ext::ExtAwi;

pub fn i128_to_usize(x: i128) -> Result<usize, String> {
    usize::try_from(x).map_err(|_| "`usize::try_from` overflow".to_owned())
}

pub fn i128_to_nonzerousize(x: i128) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(i128_to_usize(x)?).ok_or_else(|| "`NonZeroUsize::new` overflow".to_owned())
}

#[derive(Debug, Clone)]
pub enum ComponentType {
    Literal(ExtAwi),
    Variable(Vec<char>),
    Filler,
}

use ComponentType::*;

use crate::{
    chars_to_string, parse_component,
    ranges::{usize_to_i128, Usbr},
    CCMacroError,
};

#[derive(Debug, Clone)]
pub struct Component {
    pub c_type: ComponentType,
    pub range: Usbr,
}

impl Component {
    pub fn new(c_type: ComponentType, range: Usbr) -> Self {
        Self { c_type, range }
    }

    pub fn simplify(&mut self) -> Result<(), String> {
        match self.c_type.clone() {
            Literal(ref mut lit) => {
                self.range.simplify()?;
                // note: `lit` here is a reference to a clone
                self.range.simplify_literal(lit.const_as_ref())?;
                // static ranges on literals have been verified, attempt to truncate
                if let Some(x) = self.range.end.clone().and_then(|x| x.static_val()) {
                    let mut tmp = ExtAwi::zero(i128_to_nonzerousize(x)?);
                    tmp.zero_resize_assign(lit);
                    *lit = tmp;
                }
                // Note: I only truncate the start when the end is a plain static value, because
                // the subtraction from the end would introduce the possibility for an
                // underflow, and we would need yet another layer to the checks in the code gen.
                if let Some(ref end) = self.range.end {
                    if end.static_val().is_some() {
                        // attempt to truncate bits below the start
                        if let Some(x) = self.range.start.clone().and_then(|x| x.static_val()) {
                            let nz_x = i128_to_nonzerousize(x)?;
                            let w = lit.bw() - nz_x.get();
                            let mut tmp = ExtAwi::zero(nz_x);
                            tmp.field(0, lit, nz_x.get(), w);
                            *lit = tmp;
                            self.range.start.as_mut().unwrap().x = 0;
                            self.range.end.as_mut().unwrap().x -= x;
                        }
                    }
                }
                self.c_type = Literal(lit.clone());
            }
            Variable(s) => {
                if matches!(s[0], '-' | '0'..='9') {
                    let s = chars_to_string(&s);
                    match ExtAwi::from_str(&s) {
                        Ok(awi) => {
                            self.c_type = Literal(awi);
                            // does not recurse again
                            self.simplify()?;
                        }
                        Err(e) => {
                            return Err(format!(
                                "was parsed with `<ExtAwi as FromStr>::from_str(\"{}\")` which \
                                 returned SerdeError::{:?}",
                                s, e
                            ))
                        }
                    }
                } else {
                    self.range.simplify()?;
                }
            }
            Filler => {
                self.range.simplify()?;
                self.range.simplify_filler()?
            }
        }
        Ok(())
    }

    pub fn is_static_literal(&self) -> bool {
        if let Literal(..) = self.c_type {
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
        match self.c_type {
            Literal(ref lit) => {
                if let Some(ref end) = self.range.end {
                    if !end.s.is_empty() || (end.x != usize_to_i128(lit.bw()).unwrap()) {
                        return false
                    }
                }
                true
            }
            Variable(_) => self.range.end.is_none(),
            Filler => self.range.end.is_some(),
        }
    }
}

pub struct Concatenation {
    pub comps: Vec<Component>,
    pub total_bw: Option<NonZeroUsize>,
}

pub fn parse_cc(raw_cc: &[Vec<Vec<char>>]) -> Result<Vec<Concatenation>, CCMacroError> {
    let mut cc = vec![];
    for (concat_i, concat) in raw_cc.iter().enumerate() {
        let mut comps = vec![];
        for (comp_i, comp) in concat.iter().enumerate() {
            match parse_component(comp, false) {
                Ok((Some(_), _)) => todo!(),
                Ok((None, comp)) => comps.push(comp),
                Err(e) => {
                    return Err(CCMacroError {
                        concat_i: Some(concat_i),
                        comp_i: Some(comp_i),
                        error: e,
                    })
                }
            }
        }
        cc.push(Concatenation {
            comps,
            total_bw: None,
        });
    }
    Ok(cc)
}
