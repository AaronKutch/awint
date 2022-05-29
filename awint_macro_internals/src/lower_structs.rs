use std::fmt::Write;

use triple_arena::Ptr;

use crate::{
    chars_to_string, Ast, BiMap, Component, Concatenation, EitherResult, FnNames, Names, PBind,
    PCWidth, PText, PVal, PWidth, Usb,
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

pub struct Lower<'a> {
    pub binds: BiMap<PBind, Ptr<PText>, (bool, bool)>,
    pub values: BiMap<PVal, Value, bool>,
    pub widths: BiMap<PWidth, Width, bool>,
    pub cw: BiMap<PCWidth, CWidth, ()>,
    pub dynamic_width: Option<Ptr<PCWidth>>,
    pub names: Names<'a>,
    pub fn_names: FnNames<'a>,
}

impl<'a> Lower<'a> {
    pub fn new(names: Names<'a>, fn_names: FnNames<'a>) -> Self {
        Self {
            binds: BiMap::new(),
            values: BiMap::new(),
            widths: BiMap::new(),
            cw: BiMap::new(),
            dynamic_width: None,
            names,
            fn_names,
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

    pub fn lower_comp(&mut self, comp: &mut Component) -> Option<Ptr<PWidth>> {
        if comp.is_unbounded_filler() {
            return None
        }
        let res = Some(if let Some(w) = comp.range.static_width() {
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
                    .insert(Value::Bitwidth(comp.bind.unwrap()), false)
                    .either()
            };
            if comp.range.start.as_ref().unwrap().is_guaranteed_zero() {
                self.widths.insert(Width::Single(end_p_val), false).either()
            } else {
                let start_p_val = self.lower_bound(comp.range.start.as_ref().unwrap());
                comp.start = Some(start_p_val);
                self.widths
                    .insert(Width::Range(start_p_val, end_p_val), false)
                    .either()
            }
        });
        comp.width = res;
        res
    }

    pub fn lower_concat(&mut self, concat: &mut Concatenation) {
        let mut v = vec![];
        for comp in &mut concat.comps {
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
    pub fn lower_lt_checks(&mut self) -> String {
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
                    self.names.value,
                    lo.get_raw(),
                    self.names.value,
                    hi.get_raw()
                )
                .unwrap();
            }
        }
        if s.is_empty() {
            s
        } else {
            format!("{}([{}]).is_some()", self.fn_names.lt_fn, s)
        }
    }

    /// Checks that we aren't trying to squeeze the unbounded filler into
    /// negative widths
    pub fn lower_common_lt_checks(&mut self, ast: &Ast) -> String {
        let mut s = String::new();
        for concat in &ast.cc {
            if let Some(cw) = concat.cw {
                if !concat.deterministic_width {
                    if !s.is_empty() {
                        s += ",";
                    }
                    write!(s, "{}_{}", self.names.cw, cw.get_raw()).unwrap();
                }
            }
        }
        if s.is_empty() {
            s
        } else {
            format!(
                "{}({}, [{}]).is_some()",
                self.fn_names.common_lt_fn, self.names.cw, s
            )
        }
    }

    /// Checks that deterministic concat widths are equal to the common
    /// bitwidth
    pub fn lower_common_ne_checks(&mut self, ast: &Ast) -> String {
        let mut s = String::new();
        for concat in &ast.cc {
            if let Some(cw) = concat.cw {
                if concat.deterministic_width {
                    if !s.is_empty() {
                        s += ",";
                    }
                    write!(s, "{}_{}", self.names.cw, cw.get_raw()).unwrap();
                }
            }
        }
        if s.is_empty() {
            s
        } else {
            format!(
                "{}({},[{}]).is_some()",
                self.fn_names.common_ne_fn, self.names.cw, s
            )
        }
    }

    pub fn field_comp(&mut self, comp: &Component, msb_align: bool, from_buf: bool) -> String {
        let mut s = String::new();
        if let Some(width) = comp.width {
            if msb_align {
                // subtract the shift amount first
                write!(
                    s,
                    "{}-={}_{};",
                    self.names.shl,
                    self.names.width,
                    width.get_raw()
                )
                .unwrap();
            }
            if let Some(bind) = comp.bind {
                if from_buf {
                    *self.binds.a_get_mut(bind) = (true, true);
                    if let Some(start) = comp.start {
                        write!(
                            s,
                            "{}({}_{},{}_{},{},{},{}_{}){}",
                            self.fn_names.field,
                            self.names.bind,
                            bind.get_raw(),
                            self.names.value,
                            start.get_raw(),
                            self.names.awi_ref,
                            self.names.shl,
                            self.names.width,
                            width.get_raw(),
                            self.fn_names.unwrap,
                        )
                        .unwrap();
                    } else {
                        // start is zero, use field_to
                        write!(
                            s,
                            "{}({}_{},{},{},{}_{}){}",
                            self.fn_names.field_from,
                            self.names.bind,
                            bind.get_raw(),
                            self.names.awi_ref,
                            self.names.shl,
                            self.names.width,
                            width.get_raw(),
                            self.fn_names.unwrap,
                        )
                        .unwrap();
                    }
                } else {
                    self.binds.a_get_mut(bind).0 = true;
                    if let Some(start) = comp.start {
                        write!(
                            s,
                            "{}({},{},{}_{},{}_{},{}_{}){}",
                            self.fn_names.field,
                            self.names.awi_ref,
                            self.names.shl,
                            self.names.bind,
                            bind.get_raw(),
                            self.names.value,
                            start.get_raw(),
                            self.names.width,
                            width.get_raw(),
                            self.fn_names.unwrap,
                        )
                        .unwrap();
                    } else {
                        // start is zero, use field_to
                        write!(
                            s,
                            "{}({},{},{}_{},{}_{}){}",
                            self.fn_names.field_to,
                            self.names.awi_ref,
                            self.names.shl,
                            self.names.bind,
                            bind.get_raw(),
                            self.names.width,
                            width.get_raw(),
                            self.fn_names.unwrap,
                        )
                        .unwrap();
                    }
                }
            } // else is a filler, keep shift changes however
              // this runs for both fillers and nonfillers
            if !msb_align {
                // add to the shift amount afterwards
                write!(
                    s,
                    "{}+={}_{};",
                    self.names.shl,
                    self.names.width,
                    width.get_raw()
                )
                .unwrap();
            }
            writeln!(s).unwrap();
        } // else is unbounded filler, switch to other alignment
        s
    }

    pub fn field_concat(&mut self, concat: &Concatenation, from_buf: bool) -> String {
        if (concat.comps.len() == 1) && concat.comps[0].has_full_range() {
            // use copy_assign
            let sink = &concat.comps[0].bind.unwrap();
            if from_buf {
                *self.binds.a_get_mut(sink) = (true, true);
                return format!(
                    "{}({}_{},{}){};\n",
                    self.fn_names.copy_assign,
                    self.names.bind,
                    sink.get_raw(),
                    self.names.awi_ref,
                    self.fn_names.unwrap
                )
            } else {
                self.binds.a_get_mut(sink).0 = true;
                return format!(
                    "{}({},{}_{}){};\n",
                    self.fn_names.copy_assign,
                    self.names.awi_ref,
                    self.names.bind,
                    sink.get_raw(),
                    self.fn_names.unwrap
                )
            }
        }
        let mut s = format!("let mut {}=0usize;\n", self.names.shl);
        let mut lsb_i = 0;
        while lsb_i < concat.comps.len() {
            let field = self.field_comp(&concat.comps[lsb_i], false, from_buf);
            if field.is_empty() {
                // if we encounter an unbounded filler, reset and try again from the most
                // significant side
                break
            } else {
                write!(s, "{}", field).unwrap();
            }
            lsb_i += 1;
        }
        let mut msb_i = concat.comps.len() - 1;
        if msb_i > lsb_i {
            writeln!(s, "let mut {}={};", self.names.shl, self.names.cw).unwrap();
        }
        while msb_i > lsb_i {
            write!(
                s,
                "{}",
                self.field_comp(&concat.comps[msb_i], true, from_buf)
            )
            .unwrap();
            msb_i -= 1;
        }
        s
    }

    pub fn lower_fielding(
        &mut self,
        ast: &Ast,
        source_has_filler: bool,
        specified_init: bool,
        need_buffer: bool,
    ) -> String {
        let mut s = String::new();
        // FIXME if the source is full width I think buffer can be avoided
        if source_has_filler && !specified_init {
            // see explanation at top of `lowering.rs`
            for i in 1..ast.cc.len() {
                // there are some situations where this could be deduplicated
                // sink -> buf
                writeln!(s, "{}", self.field_concat(&ast.cc[i], false)).unwrap();
                // src -> buf
                writeln!(s, "{}", self.field_concat(&ast.cc[0], false)).unwrap();
                // buf -> sink
                writeln!(s, "{}", self.field_concat(&ast.cc[0], true)).unwrap();
            }
        } else if need_buffer {
            // src -> buf once
            writeln!(s, "{}", self.field_concat(&ast.cc[0], false)).unwrap();
            // buf -> sinks
            for i in 1..ast.cc.len() {
                writeln!(s, "{}", self.field_concat(&ast.cc[i], true)).unwrap();
            }
        } else {
            // direct copy assigning
            let src = ast.cc[0].comps[0].bind.unwrap();
            for i in 1..ast.cc.len() {
                let sink = ast.cc[i].comps[0].bind.unwrap();
                *self.binds.a_get_mut(sink) = (true, true);
                writeln!(
                    s,
                    "{}({}_{},{}_{}){};",
                    self.fn_names.copy_assign,
                    self.names.bind,
                    sink.get_raw(),
                    self.names.bind,
                    src.get_raw(),
                    self.fn_names.unwrap
                )
                .unwrap();
            }
        }
        s
    }
}
