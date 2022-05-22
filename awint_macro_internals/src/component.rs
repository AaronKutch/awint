use std::str::FromStr;

use awint_ext::ExtAwi;
use triple_arena::Ptr;
use ComponentType::*;

use crate::{
    chars_to_string, i128_to_nonzerousize,
    ranges::{parse_range, Usbr},
    token_tree::PText,
    usize_to_i128, Ast, CCMacroError, Delimiter, Text,
};

#[derive(Debug, Clone)]
pub enum ComponentType {
    Unparsed,
    Literal(ExtAwi),
    Variable(Vec<char>),
    Filler,
}

#[derive(Debug, Clone)]
pub struct Component {
    pub txt: Ptr<PText>,
    pub bits_txt: Option<Ptr<PText>>,
    pub range_txt: Option<Ptr<PText>>,
    pub c_type: ComponentType,
    pub range: Usbr,
}

impl Component {
    pub fn simplify(&mut self) -> Result<(), String> {
        match self.c_type.clone() {
            Unparsed => todo!(),
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
                            if x > 0 {
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
            Unparsed => todo!(),
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

/// Looks for the existence of a top level "[]" delimited group and uses the
/// last one as a bit range.
pub fn stage1(ast: &mut Ast) -> Result<(), CCMacroError> {
    // first, assign `val_txt` and `range_txt`
    for concat_i in 0..ast.cc.len() {
        for comp_i in 0..ast.cc[concat_i].comps.len() {
            let comp_txt = ast.cc[concat_i].comps[comp_i].txt;
            let len = ast.txt[comp_txt].len();
            let mut bits_len = len;
            // get top level last group
            if let Text::Group(ref mut d, p) = ast.txt[comp_txt][len - 1] {
                if let Delimiter::Bracket = d {
                    *d = Delimiter::RangeBracket;
                    ast.cc[concat_i].comps[comp_i].range_txt = Some(p);
                    bits_len -= 1;
                }
            }
            // group together for variable
            ast.cc[concat_i].comps[comp_i].bits_txt =
                Some(ast.combine_subtree(comp_txt, 0..bits_len));
        }
    }
    //"component with a bitrange that indexes nothing".to_owned(),
    //
    // the spacing check is to exclude semicolons in "::" separators
    /*if check_for_init
        && (last == 0)
        && initialization.is_none()
        && (p.as_char() == ':')
        && matches!(p.spacing(), Spacing::Alone)
    {
        initialization = Some(mem::take(&mut string));
    } else {
        string.push(p.as_char());
    }
    // specialize this case to prevent confusion
    Err("specified initialization is followed by empty component".to_owned())
    */
    for concat_i in 0..ast.cc.len() {
        for comp_i in 0..ast.cc[concat_i].comps.len() {
            // do this check after the range brackets have all been set
            let bits_txt = ast.cc[concat_i].comps[comp_i].bits_txt.unwrap();
            let mut empty = false;
            if let Text::Chars(ref s) = ast.txt[bits_txt][0] {
                if s.is_empty() {
                    empty = true;
                }
            }
            if empty {
                return Err(CCMacroError::new(
                    "there is a range but no preceeding bits".to_owned(),
                    ast.cc[concat_i].comps[comp_i].txt,
                ))
            }
            if let Some(range_txt) = ast.cc[concat_i].comps[comp_i].range_txt {
                match parse_range(ast, range_txt, true) {
                    Ok(range) => ast.cc[concat_i].comps[comp_i].range = range,
                    Err(Some(e)) => return Err(e),
                    _ => unreachable!(),
                }
            }
        }
    }

    /*
    if component_range.is_some() {
        assert!(string.is_empty());
    } else {
        component_middle = Some(string);
    }
    match (component_middle, component_range) {
        (None, None) => {
            if initialization.is_some() {
            } else {
            }
        }
        (Some(middle), Some(range)) => {
            if range.is_empty() {
                Err("has an empty index".to_owned())
            } else {
                match parse_range(&range, true) {
                    Ok(range) => Ok((
                        initialization,
                        Component::new(ComponentType::Variable(middle), range),
                    )),
                    Err(e) => Err(format!(
                        r#"could not parse range "{}": {}"#,
                        chars_to_string(&range),
                        e
                    )),
                }
            }
        }
        (Some(middle), None) => {
            // possibly a filler, check if is a range
            if let Ok(range) = parse_range(&middle, false) {
                Ok((initialization, Component::new(ComponentType::Filler, range)))
            } else {
                Ok((
                    initialization,
                    Component::new(ComponentType::Variable(middle), Usbr::unbounded()),
                ))
            }
        }
        _ => unreachable!(),
    }*/
    Ok(())
}
