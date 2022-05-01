use std::num::NonZeroUsize;

use awint_ext::ExtAwi;
use triple_arena::{ptr_trait_struct_with_gen, Arena, Ptr, PtrTrait};

pub fn i128_to_usize(x: i128) -> Result<usize, String> {
    usize::try_from(x).map_err(|_| "`usize::try_from` overflow".to_owned())
}

pub fn i128_to_nonzerousize(x: i128) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(i128_to_usize(x)?).ok_or_else(|| "`NonZeroUsize::new` overflow".to_owned())
}

#[derive(Debug, Clone)]
pub enum ComponentType {
    Literal(ExtAwi),
    Variable(String),
    Filler,
}

use ComponentType::*;

use crate::ranges::{usize_to_i128, Usbr};

#[derive(Debug, Clone)]
pub struct Component {
    pub c_type: ComponentType,
    pub range: Usbr,
}

impl Component {
    pub fn new(c_type: ComponentType, range: Usbr) -> Self {
        Self { c_type, range }
    }

    /// An error is returned only if some statically determined bound is
    /// violated, not if the simplify attempt fails
    pub fn simplify(&mut self) -> Result<(), String> {
        self.range.simplify()?;
        match self.c_type.clone() {
            Literal(ref mut lit) => {
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
            Variable(_) => (),
            Filler => self.range.simplify_filler()?,
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

// FIXME: idea: make one more token_stream function that looks for the last "[]"
// delimited group, it allows for normal Rust indexing to coexist

pub struct Concatenation {
    pub concatenation: Vec<Component>,
    pub total_bw: Option<NonZeroUsize>,
}

ptr_trait_struct_with_gen!(P0);

pub struct ParseNode {
    pub s: Vec<char>,
}
