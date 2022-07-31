use std::{mem, num::NonZeroUsize, str::FromStr};

use awint_ext::ExtAwi;
use ComponentType::*;

use crate::{
    chars_to_string, i128_to_nonzerousize, parse_range, usize_to_i128, Ast, CCMacroError,
    Delimiter, PBind, PText, PVal, PWidth, Text, Usbr,
};

#[derive(Debug, Clone)]
pub enum ComponentType {
    Unparsed,
    Literal(ExtAwi),
    Variable,
    Filler,
}

impl Default for ComponentType {
    fn default() -> Self {
        Self::Unparsed
    }
}

#[derive(Debug, Default, Clone)]
pub struct Component {
    pub txt: PText,
    pub mid_txt: Option<PText>,
    pub range_txt: Option<PText>,
    pub c_type: ComponentType,
    pub range: Usbr,
    pub bind: Option<PBind>,
    pub width: Option<PWidth>,
    pub start: Option<PVal>,
}

impl Component {
    pub fn simplify(&mut self) -> Result<(), String> {
        match self.c_type {
            Unparsed => unreachable!(),
            Literal(ref mut lit) => {
                self.range.simplify()?;
                // note: `lit` here is a reference to a clone
                self.range.simplify_literal(lit.const_as_ref())?;
                // static ranges on literals have been verified, attempt to truncate
                if let Some(ref end) = self.range.end {
                    if let Some(x) = end.static_val() {
                        let mut tmp = ExtAwi::zero(i128_to_nonzerousize(x)?);
                        tmp.zero_resize_assign(lit);
                        *lit = tmp;
                    }
                }
                // Note: I only truncate the start when the end is a plain static value, because
                // the subtraction from the end would introduce the possibility for an
                // underflow, and we would need yet another layer to the checks in the code gen.
                if let Some(ref end) = self.range.end {
                    if end.static_val().is_some() {
                        // attempt to truncate bits below the start
                        if let Some(x) = self.range.start.clone().and_then(|x| x.static_val()) {
                            if x > 0 {
                                let nz_x = i128_to_nonzerousize(x)?;
                                let w = lit.bw() - nz_x.get();
                                let mut tmp = ExtAwi::zero(NonZeroUsize::new(w).unwrap());
                                tmp.field_from(lit, nz_x.get(), w).unwrap();
                                *lit = tmp;
                                self.range.start.as_mut().unwrap().x = 0;
                                self.range.end.as_mut().unwrap().x -= x;
                            }
                        }
                    }
                }
                self.c_type = Literal(lit.clone());
            }
            Variable => {
                self.range.simplify()?;
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
            Unparsed => unreachable!(),
            Literal(ref lit) => {
                if let Some(ref end) = self.range.end {
                    if !end.s.is_empty() || (end.x != usize_to_i128(lit.bw()).unwrap()) {
                        return false
                    }
                }
                true
            }
            Variable => self.range.end.is_none(),
            Filler => self.range.end.is_some(),
        }
    }

    pub fn is_unbounded_filler(&self) -> bool {
        match self.c_type {
            Filler => self.range.end.is_none(),
            _ => false,
        }
    }
}

/// Looks for the existence of a top level "[]" delimited group and uses the
/// last one as a bit range.
pub fn stage1(ast: &mut Ast) -> Result<(), CCMacroError> {
    // first, assign `mid_txt` and `range_txt`
    for concat_i in 0..ast.cc.len() {
        for comp_i in 0..ast.cc[concat_i].comps.len() {
            let comp_txt = ast.cc[concat_i].comps[comp_i].txt;
            let len = ast.txt[comp_txt].len();
            let mut has_range = false;
            // get top level last group
            if let Text::Group(ref mut d, p) = ast.txt[comp_txt][len - 1] {
                if let Delimiter::Bracket = d {
                    *d = Delimiter::RangeBracket;
                    ast.cc[concat_i].comps[comp_i].range_txt = Some(p);
                    has_range = true;
                }
            }
            let range_txt = if has_range {
                Some(ast.txt[comp_txt].pop().unwrap())
            } else {
                None
            };
            // group together what isn't a range
            let mid_txt = mem::take(&mut ast.txt[comp_txt]);
            let mid_p = ast.txt.insert(mid_txt);
            ast.txt[comp_txt].push(Text::Group(Delimiter::None, mid_p));
            if has_range {
                ast.txt[comp_txt].push(range_txt.unwrap());
            }
            ast.cc[concat_i].comps[comp_i].mid_txt = Some(mid_p);
        }
    }

    // checking for specified initialization after ranges have been handled
    let comps_len = ast.cc[0].comps.len();
    let first_txt = ast.cc[0].comps[comps_len - 1].mid_txt.unwrap();
    let len = ast.txt[first_txt].len();
    let mut txt_i = 0;
    while txt_i < len {
        let mut is_single_colon = false;
        if let Text::Chars(ref s) = ast.txt[first_txt][txt_i] {
            if (s.len() == 1) && (s[0] == ':') {
                is_single_colon = true;
                if (txt_i + 1) < len {
                    if let Text::Chars(ref s2) = ast.txt[first_txt][txt_i + 1] {
                        if (s2.len() == 1) && (s2[0] == ':') {
                            // skip "::" separators
                            txt_i += 1;
                            is_single_colon = false;
                        }
                    }
                }
            }
        }
        if is_single_colon {
            // surgery
            if (txt_i + 1) == len {
                return Err(CCMacroError::new(
                    "specified initialization is followed by empty component".to_owned(),
                    ast.cc[0].comps[comps_len - 1].mid_txt.unwrap(),
                ))
            }
            let init_p = ast.txt.insert(ast.txt[first_txt][..txt_i].to_vec());
            ast.txt[first_txt] = ast.txt[first_txt][(txt_i + 1)..].to_vec();
            ast.txt_init = Some(init_p);
            break
        }
        txt_i += 1;
    }

    // do these checks after the range brackets have all been set
    for concat_i in 0..ast.cc.len() {
        for comp_i in 0..ast.cc[concat_i].comps.len() {
            let mid_txt = ast.cc[concat_i].comps[comp_i].mid_txt.unwrap();
            if ast.txt[mid_txt].is_empty() {
                return Err(CCMacroError::new(
                    "there is a range but no preceeding bits".to_owned(),
                    ast.cc[concat_i].comps[comp_i].txt,
                ))
            }
            if let Some(range_txt) = ast.cc[concat_i].comps[comp_i].range_txt {
                match parse_range(ast, range_txt, true) {
                    Ok(range) => ast.cc[concat_i].comps[comp_i].range = range,
                    Err(Some(e)) => return Err(e),
                    Err(None) => unreachable!(),
                }
            } else {
                // possibly a filler, check if a non-single bit range
                match parse_range(ast, mid_txt, false) {
                    Ok(range) => {
                        ast.cc[concat_i].comps[comp_i].range = range;
                        ast.cc[concat_i].comps[comp_i].c_type = Filler;
                    }
                    Err(Some(e)) => return Err(e),
                    Err(None) => (),
                }
            }
            if let Unparsed = ast.cc[concat_i].comps[comp_i].c_type {
                let mut needs_parsing = true;
                if let Text::Chars(ref s) = ast.txt[mid_txt][0] {
                    if matches!(s[0], '-' | '0'..='9') {
                        let mut s = vec![];
                        ast.chars_assign_subtree(&mut s, mid_txt);
                        let s = chars_to_string(&s);
                        match ExtAwi::from_str(&s) {
                            Ok(awi) => {
                                ast.cc[concat_i].comps[comp_i].c_type = Literal(awi);
                                needs_parsing = false;
                            }
                            Err(e) => {
                                return Err(CCMacroError::new(
                                    format!(
                                        "was parsed with `<ExtAwi as FromStr>::from_str(\"{}\")` \
                                         which returned SerdeError::{:?}",
                                        s, e
                                    ),
                                    mid_txt,
                                ))
                            }
                        }
                    }
                }
                if needs_parsing {
                    ast.cc[concat_i].comps[comp_i].c_type = Variable;
                }
            }
        }
    }
    Ok(())
}

pub fn stage2(ast: &mut Ast) -> Result<(), CCMacroError> {
    for concat in &mut ast.cc {
        for comp in &mut concat.comps {
            match comp.simplify() {
                Ok(()) => (),
                Err(e) => return Err(CCMacroError::new(e, comp.txt)),
            }
        }
    }
    Ok(())
}
