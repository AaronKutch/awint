use std::fmt::Write;

use awint_ext::ExtAwi;
use triple_arena::Ptr;

use crate::{
    chars_to_string, Ast, BiMap, Component, ComponentType, Concatenation, EitherResult, FnNames,
    Names, PBind, PCWidth, PVal, PWidth, Usb,
};

#[derive(Debug, Hash, Clone, PartialEq, Eq)]
pub enum Bind {
    Literal(ExtAwi),
    // text must be lowered by this point so that the set property works
    Txt(Vec<char>),
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

    /// In cases like `a[(a.bw() - 16)..]` we want to replace `a.bw()` with
    /// `Bits::bw(bind_to_a)` or else we run into borrow issues. This is
    /// admittedly jank, but fixes the vast majority of these kinds of cases
    /// without needing a full fledged parser for arithmetic and function calls.
    pub fn try_txt_bound_to_binding(&mut self, txt: &[char]) -> String {
        if txt.ends_with(self.fn_names.bw_call) {
            let var = txt[..(txt.len() - 5)].to_owned();
            if let Some(p) = self.binds.contains(&Bind::Txt(var)) {
                self.binds.a_get_mut(p).0 = true;
                return format!(
                    "{}({}_{})",
                    self.fn_names.get_bw,
                    self.names.bind,
                    p.get_raw()
                )
            }
        }
        chars_to_string(txt)
    }

    pub fn lower_bound(&mut self, usb: &Usb) -> Ptr<PVal> {
        if let Some(x) = usb.static_val() {
            self.values
                .insert(Value::Usize(format!("{}", x)), false)
                .either()
        } else {
            let txt = self.try_txt_bound_to_binding(&usb.s);
            if usb.x < 0 {
                self.values
                    .insert(Value::Usize(format!("{}-{}", txt, -usb.x)), false)
                    .either()
            } else if usb.x == 0 {
                self.values.insert(Value::Usize(txt), false).either()
            } else {
                self.values
                    .insert(Value::Usize(format!("{}+{}", txt, usb.x)), false)
                    .either()
            }
        }
    }

    /// Returns the width corresponding to the range of the component, and
    /// internally pushes the upperbound-bitwidth check.
    pub fn lower_comp(&mut self, comp: &mut Component) -> Option<Ptr<PWidth>> {
        if comp.is_unbounded_filler() {
            return None
        }
        // push width between the upper bound and bitwidth of variable, so that the
        // later reversal check will prevent the upper bound from being beyond the
        // bitwidth
        let need_extra_check =
            comp.bind.is_some() && (!comp.has_full_range()) && (!comp.is_static_literal());
        let res = Some(if let Some(w) = comp.range.static_width() {
            if !comp.range.start.as_ref().unwrap().is_guaranteed_zero() {
                let start_p_val = self.lower_bound(comp.range.start.as_ref().unwrap());
                comp.start = Some(start_p_val);
            }
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
    pub fn lower_le_checks(&mut self) -> String {
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
            format!("{}([{}]).is_some()", self.fn_names.le_fn, s)
        }
    }

    /// Checks that we aren't trying to squeeze unbounded fillers into
    /// negative widths for nondeterministic cases and that deterministic concat
    /// widths are equal to the common bitwidth
    pub fn lower_common_checks(&mut self, ast: &Ast) -> String {
        let mut ge = String::new();
        let mut eq = String::new();
        for concat in &ast.cc {
            if (concat.comps.len() == 1) && concat.comps[0].is_unbounded_filler() {
                continue
            }
            if concat.static_width.is_none() {
                let cw = concat.cw.unwrap();
                *self.cw.a_get_mut(cw) = true;
                if concat.deterministic_width {
                    let mut set_dynamic_width = false;
                    if self.dynamic_width.is_none() {
                        self.dynamic_width = Some(cw);
                        set_dynamic_width = true;
                    }
                    // we can avoid comparing the same value against itself, however the second
                    // condition handles cases like `inlawi!(a; ..64)` where the dynamic width might
                    // not be equal to the static width we expect
                    if (!set_dynamic_width) || ast.common_bw.is_some() {
                        if !eq.is_empty() {
                            eq += ",";
                        }
                        write!(eq, "{}_{}", self.names.cw, cw.get_raw()).unwrap();
                    }
                } else {
                    if !ge.is_empty() {
                        ge += ",";
                    }
                    write!(ge, "{}_{}", self.names.cw, cw.get_raw()).unwrap();
                }
            }
        }
        if eq.is_empty() && ge.is_empty() {
            String::new()
        } else {
            format!(
                "{}({},[{}],[{}]).is_some()",
                self.fn_names.common_fn, self.names.cw, ge, eq
            )
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn write_field(
        &mut self,
        s: &mut String,
        lhs: &str,
        to: Option<&str>,
        rhs: &str,
        from: Option<&str>,
        width: &str,
        width1: bool,
    ) {
        if width1 {
            // #[inline] is on Bits::field_bit, Bits::get, and Bits::set so this is ok
            match (to, from) {
                (Some(to), Some(from)) => write!(
                    s,
                    "{}({},{},{},{})",
                    self.fn_names.field_bit, lhs, to, rhs, from
                )
                .unwrap(),
                (Some(to), None) => {
                    write!(s, "{}({},{},{},0)", self.fn_names.field_bit, lhs, to, rhs).unwrap()
                }
                (None, Some(from)) => {
                    write!(s, "{}({},0,{},{})", self.fn_names.field_bit, lhs, rhs, from).unwrap()
                }
                (None, None) => {
                    write!(s, "{}({},0,{},0)", self.fn_names.field_bit, lhs, rhs).unwrap()
                }
            }
            write!(s, "{};", self.fn_names.unwrap).unwrap();
        } else {
            match (to, from) {
                (Some(to), Some(from)) => {
                    write!(s, "{}({},{},{},{}", self.fn_names.field, lhs, to, rhs, from).unwrap()
                }
                (Some(to), None) => {
                    write!(s, "{}({},{},{}", self.fn_names.field_to, lhs, to, rhs).unwrap()
                }
                (None, Some(from)) => {
                    write!(s, "{}({},{},{}", self.fn_names.field_from, lhs, rhs, from).unwrap()
                }
                (None, None) => write!(s, "{}({},{}", self.fn_names.field_width, lhs, rhs).unwrap(),
            }
            write!(s, ",{}){};", width, self.fn_names.unwrap).unwrap();
        }
    }

    pub fn field_comp(
        &mut self,
        s: &mut String,
        comp: &Component,
        msb_align: bool,
        from_buf: bool,
        first_in_align: bool,
        last_in_align: bool,
    ) {
        let width = comp.width.unwrap();
        let mut width1 = false;
        if let Some(w) = comp.range.static_width() {
            if w == 1 {
                width1 = true;
            }
        }
        let width_s = format!("{}_{}", self.names.width, width.get_raw());
        if msb_align {
            // subtract the shift amount first
            if first_in_align {
                write!(
                    s,
                    "let mut {}={}-{}_{};",
                    self.names.shl,
                    self.names.cw,
                    self.names.width,
                    width.get_raw()
                )
                .unwrap();
            } else {
                write!(
                    s,
                    "{}-={}_{};",
                    self.names.shl,
                    self.names.width,
                    width.get_raw()
                )
                .unwrap();
            }
        }
        self.widths.a_get_mut(width).1 = true;
        if let Some(bind) = comp.bind {
            let bind_s = format!("{}_{}", self.names.bind, bind.get_raw());
            if from_buf {
                *self.binds.a_get_mut(bind) = (true, true);
                if let Some(start) = comp.start {
                    *self.values.a_get_mut(start) = true;
                }
                let start_s = if let Some(start) = comp.start {
                    format!("{}_{}", self.names.value, start.get_raw())
                } else {
                    String::new()
                };
                self.write_field(
                    s,
                    &bind_s,
                    if start_s.is_empty() {
                        None
                    } else {
                        Some(&start_s)
                    },
                    self.names.awi_ref,
                    if (!msb_align) && first_in_align {
                        None
                    } else {
                        Some(self.names.shl)
                    },
                    &width_s,
                    width1,
                );
            } else {
                self.binds.a_get_mut(bind).0 = true;
                if let Some(start) = comp.start {
                    *self.values.a_get_mut(start) = true;
                }
                let start_s = if let Some(start) = comp.start {
                    format!("{}_{}", self.names.value, start.get_raw())
                } else {
                    String::new()
                };
                self.write_field(
                    s,
                    self.names.awi_ref,
                    if (!msb_align) && first_in_align {
                        None
                    } else {
                        Some(self.names.shl)
                    },
                    &bind_s,
                    if start_s.is_empty() {
                        None
                    } else {
                        Some(&start_s)
                    },
                    &width_s,
                    width1,
                );
            }
        } // else is a filler, keep shift changes however
          // this runs for both fillers and nonfillers
        if !(msb_align || last_in_align) {
            // add to the shift amount afterwards
            if first_in_align {
                write!(
                    s,
                    "let mut {}={}_{};",
                    self.names.shl,
                    self.names.width,
                    width.get_raw()
                )
                .unwrap();
            } else {
                write!(
                    s,
                    "{}+={}_{};",
                    self.names.shl,
                    self.names.width,
                    width.get_raw()
                )
                .unwrap();
            }
        }
        writeln!(s).unwrap();
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
        let mut s = String::new();
        let mut lsb_i = 0;
        while lsb_i < concat.comps.len() {
            if concat.comps[lsb_i].width.is_none() {
                break
            }
            self.field_comp(
                &mut s,
                &concat.comps[lsb_i],
                false,
                from_buf,
                lsb_i == 0,
                (lsb_i + 1) == concat.comps.len(),
            );
            lsb_i += 1;
        }
        let mut msb_i = concat.comps.len() - 1;
        while msb_i > lsb_i {
            if concat.comps[msb_i].width.is_none() {
                break
            }
            self.field_comp(
                &mut s,
                &concat.comps[msb_i],
                true,
                from_buf,
                msb_i == (concat.comps.len() - 1),
                (msb_i - 1) == lsb_i,
            );
            msb_i -= 1;
        }
        s
    }

    pub fn lower_fielding(
        &mut self,
        ast: &Ast,
        source_has_filler: bool,
        need_buffer: bool,
    ) -> String {
        let mut s = String::new();
        if source_has_filler && ast.txt_init.is_none() {
            // see explanation at top of `lowering.rs`
            for i in 1..ast.cc.len() {
                // there are some situations where this could be deduplicated
                // sink -> buf
                writeln!(s, "{}", self.field_concat(&ast.cc[i], false)).unwrap();
                // src -> buf
                writeln!(s, "{}", self.field_concat(&ast.cc[0], false)).unwrap();
                // buf -> sink
                writeln!(s, "{}", self.field_concat(&ast.cc[i], true)).unwrap();
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
            if let Some(src) = ast.cc[0].comps[0].bind {
                self.binds.a_get_mut(src).0 = true;
                for i in 1..ast.cc.len() {
                    if let Some(sink) = ast.cc[i].comps[0].bind {
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
            } // else cases like `extawi!(umax: ..r)` or `extawi!(umax: ..;
              // ..8)`
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
        mut lit_construction_fn: F,
    ) -> String {
        let mut s = String::new();
        for (p_b, (bind, (used, mutable))) in self.binds.arena() {
            if *used {
                match bind {
                    Bind::Literal(ref awi) => {
                        writeln!(
                            s,
                            "let {}_{}:{}=&{};",
                            self.names.bind,
                            p_b.get_raw(),
                            self.fn_names.bits_ref,
                            (lit_construction_fn)(ExtAwi::from_bits(awi))
                        )
                        .unwrap();
                    }
                    Bind::Txt(chars) => {
                        let chars = chars_to_string(chars);
                        // note: we can't use extra braces in the bindings or else the E0716
                        // workaround doesn't work
                        if *mutable {
                            writeln!(
                                s,
                                "let {}_{}:{}=&mut {};",
                                self.names.bind,
                                p_b.get_raw(),
                                self.fn_names.mut_bits_ref,
                                chars
                            )
                            .unwrap();
                        } else {
                            writeln!(
                                s,
                                "let {}_{}:{}=&{};",
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
