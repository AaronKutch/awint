use std::fmt::Write;

use triple_arena::Ptr;

use crate::{
    chars_to_string, Ast, BiMap, Component, Concatenation, EitherResult, Names, PBind, PCWidth,
    PText, PVal, PWidth, Usb,
};

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum Value {
    /// value comes from bitwidth of binding
    Bitwidth(Ptr<PBind>),
    /// value comes from `usize` literal or variable
    Usize(String),
}

/// Bitwidth as described by a single value or one value minus another
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum Width {
    Single(Ptr<PVal>),
    Range(Ptr<PVal>, Ptr<PVal>),
}

/// For concatenation widths
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub struct CWidth(Vec<Ptr<PWidth>>);

pub struct Lower {
    pub binds: BiMap<PBind, Ptr<PText>, (bool, bool)>,
    pub values: BiMap<PVal, Value, bool>,
    pub widths: BiMap<PWidth, Width, bool>,
    pub cw: BiMap<PCWidth, CWidth, ()>,
    pub dynamic_width: Option<Ptr<PCWidth>>,
}

impl Lower {
    pub fn new() -> Self {
        Self {
            binds: BiMap::new(),
            values: BiMap::new(),
            widths: BiMap::new(),
            cw: BiMap::new(),
            dynamic_width: None,
        }
    }

    pub fn lower_bound(&mut self, usb: &Usb) -> Ptr<PVal> {
        if let Some(x) = usb.static_val() {
            self.values
                .insert(Value::Usize(format!("{}", x)), false)
                .either()
        } else if usb.x < 0 {
            self.values
                .insert(
                    Value::Usize(format!("{}-{}", chars_to_string(&usb.s), -usb.x)),
                    false,
                )
                .either()
        } else {
            self.values
                .insert(
                    Value::Usize(format!("{}+{}", chars_to_string(&usb.s), usb.x)),
                    false,
                )
                .either()
        }
    }

    pub fn lower_comp(&mut self, comp: &Component) -> Option<Ptr<PWidth>> {
        if comp.is_unbounded_filler() {
            return None
        }
        Some(if let Some(w) = comp.range.static_width() {
            let p_val = self
                .values
                .insert(Value::Usize(format!("{}", w)), false)
                .either();
            self.widths.insert(Width::Single(p_val), false).either()
        } else {
            let end_p_val = if let Some(ref end) = comp.range.end {
                self.lower_bound(end)
            } else {
                // need information from component
                self.values
                    .insert(Value::Bitwidth(comp.binding.unwrap()), false)
                    .either()
            };
            if comp.range.start.as_ref().unwrap().is_guaranteed_zero() {
                self.widths.insert(Width::Single(end_p_val), false).either()
            } else {
                let start_p_val = self.lower_bound(comp.range.start.as_ref().unwrap());
                self.widths
                    .insert(Width::Range(start_p_val, end_p_val), false)
                    .either()
            }
        })
    }

    pub fn lower_concat(&mut self, concat: &mut Concatenation) {
        let mut v = vec![];
        for comp in &concat.comps {
            if let Some(w) = self.lower_comp(comp) {
                v.push(w);
            }
        }
        // note that in the `is_some` case, we want to lower the components anyway
        // because of component range checks that are still needed in some circumstances
        if concat.total_bw.is_none() {
            concat.cw = Some(self.cw.insert(CWidth(v), ()).either());
        }
    }

    /// Checks that ranges aren't reversed
    pub fn lower_lt_checks(&mut self, lt_fn: &str, names: Names) -> String {
        let mut s = String::new();
        for (width, used) in self.widths.arena_mut().vals_mut() {
            if let Width::Range(lo, hi) = width {
                *used = true;
                if !s.is_empty() {
                    s += ",";
                }
                write!(
                    s,
                    "({}_{},{}_{})",
                    names.value,
                    lo.get_raw(),
                    names.value,
                    hi.get_raw()
                )
                .unwrap();
            }
        }
        if s.is_empty() {
            s
        } else {
            format!("{}([{}]).is_some()", lt_fn, s)
        }
    }

    /// Checks that we aren't trying to squeeze the unbounded filler into
    /// negative widths
    pub fn lower_common_lt_checks(
        &mut self,
        ast: &Ast,
        common_lt_fn: &str,
        names: Names,
    ) -> String {
        let mut s = String::new();
        for concat in &ast.cc {
            if let Some(cw) = concat.cw {
                if !concat.deterministic_width {
                    if !s.is_empty() {
                        s += ",";
                    }
                    write!(s, "{}_{}", names.cw, cw.get_raw()).unwrap();
                }
            }
        }
        if s.is_empty() {
            s
        } else {
            format!("{}({}, [{}]).is_some()", common_lt_fn, names.cw, s)
        }
    }

    /// Checks that deterministic concat widths are equal to the common
    /// bitwidth
    pub fn lower_common_ne_checks(
        &mut self,
        ast: &Ast,
        common_ne_fn: &str,
        names: Names,
    ) -> String {
        let mut s = String::new();
        for concat in &ast.cc {
            if let Some(cw) = concat.cw {
                if concat.deterministic_width {
                    if !s.is_empty() {
                        s += ",";
                    }
                    write!(s, "{}_{}", names.cw, cw.get_raw()).unwrap();
                }
            }
        }
        if s.is_empty() {
            s
        } else {
            format!("{}({}, [{}]).is_some()", common_ne_fn, names.cw, s)
        }
    }
}

impl Default for Lower {
    fn default() -> Self {
        Self::new()
    }
}
