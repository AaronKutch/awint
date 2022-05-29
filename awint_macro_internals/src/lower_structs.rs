use triple_arena::Ptr;

use crate::{
    chars_to_string, BiMap, Component, Concatenation, EitherResult, PBind, PSumWidth, PText, PVal,
    PWidth, Usb,
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
pub struct SumWidths(Vec<Ptr<PWidth>>);

pub struct Lower {
    pub binds: BiMap<PBind, Ptr<PText>, (bool, bool)>,
    pub values: BiMap<PVal, Value, bool>,
    pub widths: BiMap<PWidth, Width, bool>,
    pub sum_widths: BiMap<PSumWidth, SumWidths, bool>,
    pub static_width: Option<Ptr<PVal>>,
    pub dynamic_width: Option<Ptr<PSumWidth>>,
}

impl Lower {
    pub fn new() -> Self {
        Self {
            binds: BiMap::new(),
            values: BiMap::new(),
            widths: BiMap::new(),
            sum_widths: BiMap::new(),
            static_width: None,
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
            concat.sum_widths = Some(self.sum_widths.insert(SumWidths(v), false).either());
        }
    }
}

impl Default for Lower {
    fn default() -> Self {
        Self::new()
    }
}
