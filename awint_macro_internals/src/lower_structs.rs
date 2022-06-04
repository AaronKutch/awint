use std::fmt::Write;

use awint_ext::ExtAwi;
use triple_arena::Ptr;

use crate::{
    chars_to_string, Ast, BiMap, Component, ComponentType, Concatenation, EitherResult, FnNames,
    Names, PBind, PCWidth, PText, PVal, PWidth, Usb,
};

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum Bind {
    Literal(ExtAwi),
    Txt(Ptr<PText>),
}

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
    /// The first bool is if the binding is used, the second is for if it needs
    /// to be mutable
    pub binds: BiMap<PBind, Bind, (bool, bool)>,
    /// The bool is if the value is used
    pub values: BiMap<PVal, Value, bool>,
    /// The first bool is if the width is used for lt checks, the second if it
    /// needs to be assigned to a `let` binding
    pub widths: BiMap<PWidth, Width, (bool, bool)>,
    // the bool is if the cw is used
    pub cw: BiMap<PCWidth, CWidth, bool>,
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

    // Returns the width corresponding to the range of the component, and internally
    // pushes the upperbound-bitwidth check.
    pub fn lower_comp(&mut self, comp: &mut Component) -> Option<Ptr<PWidth>> {
        if comp.is_unbounded_filler() {
            return None
        }
        // push width between the upper bound and bitwidth of variable, so that the
        // later reversal check will prevent the upper bound from being beyond the
        // bitwidth
        let need_extra_check = !(comp.has_full_range() || comp.is_static_literal());
        let res = Some(if let Some(w) = comp.range.static_width() {
            let p_val = self
                .values
                .insert(Value::Usize(format!("{}", w)), false)
                .either();
            if need_extra_check {
                if let Some(ref end) = comp.range.end {
                    // range end is not the same as variable end, need to check
                    let end_p_val = self.lower_bound(end);
                    let var_end_p_val = self
                        .values
                        .insert(Value::Bitwidth(comp.bind.unwrap()), false)
                        .either();
                    self.widths
                        .insert(Width::Range(end_p_val, var_end_p_val), (true, false))
                        .either();
                };
            }
            self.widths
                .insert(Width::Single(p_val), (false, false))
                .either()
        } else {
            let end_p_val = if let Some(ref end) = comp.range.end {
                let end_p_val = self.lower_bound(end);
                if need_extra_check {
                    // range end is not the same as variable end, need to check
                    let var_end_p_val = self
                        .values
                        .insert(Value::Bitwidth(comp.bind.unwrap()), false)
                        .either();
                    self.widths
                        .insert(Width::Range(end_p_val, var_end_p_val), (true, false))
                        .either();
                }
                end_p_val
            } else {
                // need information from component
                self.values
                    .insert(Value::Bitwidth(comp.bind.unwrap()), false)
                    .either()
            };
            if comp.range.start.as_ref().unwrap().is_guaranteed_zero() {
                self.widths
                    .insert(Width::Single(end_p_val), (false, false))
                    .either()
            } else {
                let start_p_val = self.lower_bound(comp.range.start.as_ref().unwrap());
                comp.start = Some(start_p_val);
                self.widths
                    .insert(Width::Range(start_p_val, end_p_val), (true, false))
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
        concat.cw = Some(self.cw.insert(CWidth(v), false).either());
    }

    /// Checks that ranges aren't reversed and the upper bounds are not beyond
    /// bitwidths
    pub fn lower_lt_checks(&mut self) -> String {
        let mut s = String::new();
        for (width, used) in self.widths.arena_mut().vals_mut() {
            if used.0 {
                if let Width::Range(lo, hi) = width {
                    if !s.is_empty() {
                        s += ",";
                    }
                    *self.values.a_get_mut(*lo) = true;
                    *self.values.a_get_mut(*hi) = true;
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
        }
        if s.is_empty() {
            s
        } else {
            format!("{}([{}]).is_some()", self.fn_names.lt_fn, s)
        }
    }

    /// Checks that we aren't trying to squeeze unbounded fillers into
    /// negative widths for nondeterministic cases and that deterministic concat
    /// widths are equal to the common bitwidth
    pub fn lower_common_checks(&mut self, ast: &Ast) -> String {
        let mut ne = String::new();
        let mut lt = String::new();
        for concat in &ast.cc {
            if concat.static_width.is_none() {
                let cw = concat.cw.unwrap();
                *self.cw.a_get_mut(cw) = true;
                if concat.deterministic_width {
                    if !ne.is_empty() {
                        ne += ",";
                    }
                    write!(ne, "{}_{}", self.names.cw, cw.get_raw()).unwrap();
                } else {
                    if !lt.is_empty() {
                        lt += ",";
                    }
                    write!(lt, "{}_{}", self.names.cw, cw.get_raw()).unwrap();
                }
            }
        }
        match (ne.is_empty(), lt.is_empty()) {
            (true, true) => String::new(),
            (true, false) => format!(
                "{}({},[{}]).is_some()",
                self.fn_names.common_lt_fn, self.names.cw, lt
            ),
            (false, true) => format!(
                "{}({},[{}]).is_some()",
                self.fn_names.common_ne_fn, self.names.cw, ne
            ),
            (false, false) => format!(
                "{}({},[{}]).is_some() && {}({},[{}]).is_some()",
                self.fn_names.common_lt_fn,
                self.names.cw,
                lt,
                self.fn_names.common_ne_fn,
                self.names.cw,
                ne,
            ),
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
            self.widths.a_get_mut(width).1 = true;
            if let Some(bind) = comp.bind {
                if from_buf {
                    *self.binds.a_get_mut(bind) = (true, true);
                    if let Some(start) = comp.start {
                        write!(
                            s,
                            "{}({}_{},{}_{},{},{},{}_{}){};",
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
                            "{}({}_{},{},{},{}_{}){};",
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
                            "{}({},{},{}_{},{}_{},{}_{}){};",
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
                            "{}({},{},{}_{},{}_{}){};",
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
            if matches!(&concat.comps[0].c_type, ComponentType::Filler) {
                return String::new()
            }
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
            self.binds.a_get_mut(src).0 = true;
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

    pub fn lower_cws(&mut self) -> String {
        let mut s = String::new();
        for (p_cw, (cw, used)) in self.cw.arena() {
            if *used {
                write!(s, "let {}_{}=", self.names.cw, p_cw.get_raw()).unwrap();
                for (i, w) in cw.0.iter().enumerate() {
                    self.widths.a_get_mut(w).1 = true;
                    if i != 0 {
                        write!(s, "+").unwrap();
                    }
                    write!(s, "{}_{}", self.names.width, w.get_raw()).unwrap();
                }
                writeln!(s, ";").unwrap();
            }
        }
        s
    }

    pub fn lower_widths(&mut self) -> String {
        let mut s = String::new();
        for (p_w, (w, used)) in self.widths.arena() {
            if used.1 {
                match w {
                    Width::Single(v) => {
                        *self.values.a_get_mut(v) = true;
                        writeln!(
                            s,
                            "let {}_{}={}_{};",
                            self.names.width,
                            p_w.get_raw(),
                            self.names.value,
                            v.get_raw()
                        )
                        .unwrap();
                    }
                    Width::Range(v0, v1) => {
                        *self.values.a_get_mut(v0) = true;
                        *self.values.a_get_mut(v1) = true;
                        writeln!(
                            s,
                            "let {}_{}={}_{}-{}_{};",
                            self.names.width,
                            p_w.get_raw(),
                            self.names.value,
                            v1.get_raw(),
                            self.names.value,
                            v0.get_raw()
                        )
                        .unwrap();
                    }
                }
            }
        }
        s
    }

    pub fn lower_values(&mut self) -> String {
        let mut s = String::new();
        for (p_v, (v, used)) in self.values.arena() {
            if *used {
                match v {
                    Value::Bitwidth(b) => {
                        self.binds.a_get_mut(b).0 = true;
                        writeln!(
                            s,
                            "let {}_{}={}({}_{});",
                            self.names.value,
                            p_v.get_raw(),
                            self.fn_names.get_bw,
                            self.names.bind,
                            b.get_raw()
                        )
                        .unwrap();
                    }
                    Value::Usize(string) => {
                        writeln!(
                            s,
                            "let {}_{}:usize={};",
                            self.names.value,
                            p_v.get_raw(),
                            string
                        )
                        .unwrap();
                    }
                }
            }
        }
        s
    }

    pub fn lower_bindings<F: FnMut(ExtAwi) -> String>(
        &mut self,
        ast: &Ast,
        mut lit_construction_fn: F,
    ) -> String {
        let mut s = String::new();
        for (p_b, (bind, (used, mutable))) in self.binds.arena() {
            if *used {
                match bind {
                    Bind::Literal(ref awi) => {
                        writeln!(
                            s,
                            "let {}_{}=&{};",
                            self.names.bind,
                            p_b.get_raw(),
                            (lit_construction_fn)(ExtAwi::from_bits(awi))
                        )
                        .unwrap();
                    }
                    Bind::Txt(txt) => {
                        let mut chars = vec![];
                        ast.chars_assign_subtree(&mut chars, *txt);
                        let chars = chars_to_string(&chars);
                        if *mutable {
                            writeln!(
                                s,
                                "let {}_{}:{}=&mut{{{}}};",
                                self.names.bind,
                                p_b.get_raw(),
                                self.fn_names.mut_bits_ref,
                                chars
                            )
                            .unwrap();
                        } else {
                            writeln!(
                                s,
                                "let {}_{}:{}=&{{{}}};",
                                self.names.bind,
                                p_b.get_raw(),
                                self.fn_names.bits_ref,
                                chars
                            )
                            .unwrap();
                        }
                    }
                }
            }
        }
        s
    }
}
