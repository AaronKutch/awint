use std::fmt::Write;

use awint_ext::{awint_core::OrdBits, Awi};
use triple_arena::{OrdArena, Ptr};

use crate::{
    chars_to_string, Ast, Component, ComponentType, Concatenation, FillerAlign, FnNames, Names,
    PBind, PCWidth, PVal, PWidth, Usb,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Bind {
    Literal(OrdBits<Awi>),
    // text must be lowered by this point so that the set property works
    Txt(Vec<char>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    /// value comes from bitwidth of binding
    Bitwidth(PBind),
    /// value comes from `usize` literal or variable
    Usize(String),
}

/// Bitwidth as described by a single value or one value minus another
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Width {
    Single(PVal),
    Range(PVal, PVal),
}

/// For concatenation widths
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CWidth(Vec<PWidth>);

pub struct Lower<'a> {
    /// The first bool is if the binding is used, the second is for if it needs
    /// to be mutable
    pub binds: OrdArena<PBind, Bind, (bool, bool)>,
    /// The bool is if the value is used
    pub values: OrdArena<PVal, Value, bool>,
    /// The first bool is if the width is used for lt checks, the second if it
    /// needs to be assigned to a `let` binding
    pub widths: OrdArena<PWidth, Width, (bool, bool)>,
    // the bool is if the cw is used
    pub cw: OrdArena<PCWidth, CWidth, bool>,
    pub dynamic_width: Option<PCWidth>,
    pub names: Names<'a>,
    pub fn_names: FnNames<'a>,
}

impl<'a> Lower<'a> {
    pub fn new(names: Names<'a>, fn_names: FnNames<'a>) -> Self {
        Self {
            binds: OrdArena::new(),
            values: OrdArena::new(),
            widths: OrdArena::new(),
            cw: OrdArena::new(),
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
            if let Some(p) = self.binds.find_key(&Bind::Txt(var)) {
                self.binds.get_val_mut(p).unwrap().0 = true;
                return format!("{}({}_{})", self.fn_names.get_bw, self.names.bind, p.inx())
            }
        }
        chars_to_string(txt)
    }

    pub fn lower_bound(&mut self, usb: &Usb) -> PVal {
        if let Some(x) = usb.static_val() {
            self.values.insert(Value::Usize(format!("{x}")), false).0
        } else {
            let txt = self.try_txt_bound_to_binding(&usb.s);
            if usb.x < 0 {
                self.values
                    .insert(
                        Value::Usize(format!("{}({},{})", self.fn_names.usize_sub, txt, -usb.x)),
                        false,
                    )
                    .0
            } else if usb.x == 0 {
                self.values.insert(Value::Usize(txt), false).0
            } else {
                self.values
                    .insert(
                        Value::Usize(format!("{}({},{})", self.fn_names.usize_add, txt, usb.x)),
                        false,
                    )
                    .0
            }
        }
    }

    /// Returns the width corresponding to the range of the component, and
    /// internally pushes the upperbound-bitwidth check.
    pub fn lower_comp(&mut self, comp: &mut Component) -> Option<PWidth> {
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
            let p_val = self.values.insert(Value::Usize(format!("{w}")), false).0;
            if need_extra_check {
                if let Some(ref end) = comp.range.end {
                    // range end is not the same as variable end, need to check
                    let end_p_val = self.lower_bound(end);
                    let var_end_p_val = self
                        .values
                        .insert(Value::Bitwidth(comp.bind.unwrap()), false)
                        .0;
                    let _ = self
                        .widths
                        .insert(Width::Range(end_p_val, var_end_p_val), (true, false));
                };
            }
            self.widths.insert(Width::Single(p_val), (false, false)).0
        } else {
            let end_p_val = if let Some(ref end) = comp.range.end {
                let end_p_val = self.lower_bound(end);
                if need_extra_check {
                    // range end is not the same as variable end, need to check
                    let var_end_p_val = self
                        .values
                        .insert(Value::Bitwidth(comp.bind.unwrap()), false)
                        .0;
                    let _ = self
                        .widths
                        .insert(Width::Range(end_p_val, var_end_p_val), (true, false));
                }
                end_p_val
            } else {
                // need information from component
                self.values
                    .insert(Value::Bitwidth(comp.bind.unwrap()), false)
                    .0
            };
            if comp.range.start.as_ref().unwrap().is_guaranteed_zero() {
                self.widths
                    .insert(Width::Single(end_p_val), (false, false))
                    .0
            } else {
                let start_p_val = self.lower_bound(comp.range.start.as_ref().unwrap());
                comp.start = Some(start_p_val);
                self.widths
                    .insert(Width::Range(start_p_val, end_p_val), (true, false))
                    .0
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
        // `v` can be empty in cases like `extawi!(umax: ..; ..8)`
        if !v.is_empty() {
            concat.cw = Some(self.cw.insert(CWidth(v), false).0);
        }
    }

    /// Checks that ranges aren't reversed and the upper bounds are not beyond
    /// bitwidths. Returns `true` and "0;0" for both strings if there are no
    /// checks (for `awint_dag` purposes)
    pub fn lower_le_checks(&mut self) -> (bool, String, String) {
        let mut s0 = String::new();
        let mut s1 = String::new();
        for (_, width, used) in self.widths.iter() {
            if used.0 {
                if let Width::Range(lo, hi) = width {
                    if !s0.is_empty() {
                        s0 += ",";
                        s1 += ",";
                    }
                    *self.values.get_val_mut(*lo).unwrap() = true;
                    *self.values.get_val_mut(*hi).unwrap() = true;
                    write!(s0, "{}_{}", self.names.value, lo.inx(),).unwrap();
                    write!(s1, "{}_{}", self.names.value, hi.inx()).unwrap();
                }
            }
        }
        if s0.is_empty() {
            (true, "0;0".to_owned(), "0;0".to_owned())
        } else {
            (false, s0, s1)
        }
    }

    /// Checks that we aren't trying to squeeze unbounded fillers into
    /// negative widths for nondeterministic cases and that deterministic concat
    /// widths are equal to the common bitwidth. Returns `true` and "0;0" for
    /// both strings if there are no checks (for `awint_dag` purposes)
    pub fn lower_common_checks(&mut self, ast: &Ast) -> (bool, String, String) {
        if matches!(
            ast.overall_alignment,
            FillerAlign::Single | FillerAlign::Lsb | FillerAlign::Msb
        ) {
            // always infallible except potentially with respect to 0 bitwidth return
            // situations which is handled elsewhere
            return (true, "0;0".to_owned(), "0;0".to_owned())
        }
        let mut ge = String::new();
        let mut eq = String::new();
        for concat in &ast.cc {
            if concat.static_width.is_none() {
                if let Some(cw) = concat.cw {
                    *self.cw.get_val_mut(cw).unwrap() = true;
                    if concat.deterministic_width {
                        let mut set_dynamic_width = false;
                        if self.dynamic_width.is_none() {
                            self.dynamic_width = Some(cw);
                            set_dynamic_width = true;
                        }
                        // we can avoid comparing the same value against itself, however the second
                        // condition handles cases like `inlawi!(a; ..64)` where the dynamic width
                        // might not be equal to the static width we expect
                        if (!set_dynamic_width) || ast.common_bw.is_some() {
                            if !eq.is_empty() {
                                eq += ",";
                            }
                            write!(eq, "{}_{}", self.names.cw, cw.inx()).unwrap();
                        }
                    } else {
                        if !ge.is_empty() {
                            ge += ",";
                        }
                        write!(ge, "{}_{}", self.names.cw, cw.inx()).unwrap();
                    }
                }
            }
        }
        if eq.is_empty() && ge.is_empty() {
            (true, "0;0".to_owned(), "0;0".to_owned())
        } else {
            if eq.is_empty() {
                eq = "0;0".to_owned();
            }
            if ge.is_empty() {
                ge = "0;0".to_owned();
            }
            (false, ge, eq)
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
                    "let _ = {}({},{},{},{});",
                    self.fn_names.field_bit, lhs, to, rhs, from
                )
                .unwrap(),
                (Some(to), None) => write!(
                    s,
                    "let _ = {}({},{},{},0);",
                    self.fn_names.field_bit, lhs, to, rhs
                )
                .unwrap(),
                (None, Some(from)) => write!(
                    s,
                    "let _ = {}({},0,{},{});",
                    self.fn_names.field_bit, lhs, rhs, from
                )
                .unwrap(),
                (None, None) => write!(
                    s,
                    "let _ = {}({},0,{},0);",
                    self.fn_names.field_bit, lhs, rhs
                )
                .unwrap(),
            }
        } else {
            match (to, from) {
                (Some(to), Some(from)) => write!(
                    s,
                    "let _ = {}({},{},{},{}",
                    self.fn_names.field, lhs, to, rhs, from
                )
                .unwrap(),
                (Some(to), None) => write!(
                    s,
                    "let _ = {}({},{},{}",
                    self.fn_names.field_to, lhs, to, rhs
                )
                .unwrap(),
                (None, Some(from)) => write!(
                    s,
                    "let _ = {}({},{},{}",
                    self.fn_names.field_from, lhs, rhs, from
                )
                .unwrap(),
                (None, None) => {
                    write!(s, "let _ = {}({},{}", self.fn_names.field_width, lhs, rhs).unwrap()
                }
            }
            write!(s, ",{width});").unwrap();
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
        let width_s = format!("{}_{}", self.names.width, width.inx());
        if msb_align {
            // subtract the shift amount first
            if first_in_align {
                write!(
                    s,
                    "let mut {}={}({},{}_{});",
                    self.names.shl,
                    self.fn_names.usize_sub,
                    self.names.cw,
                    self.names.width,
                    width.inx()
                )
                .unwrap();
            } else {
                write!(
                    s,
                    "{}={}({},{}_{});",
                    self.names.shl,
                    self.fn_names.usize_sub,
                    self.names.shl,
                    self.names.width,
                    width.inx()
                )
                .unwrap();
            }
        }
        self.widths.get_val_mut(width).unwrap().1 = true;
        if let Some(bind) = comp.bind {
            let bind_s = format!("{}_{}", self.names.bind, bind.inx());
            if from_buf {
                *self.binds.get_val_mut(bind).unwrap() = (true, true);
                if let Some(start) = comp.start {
                    *self.values.get_val_mut(start).unwrap() = true;
                }
                let start_s = if let Some(start) = comp.start {
                    format!("{}_{}", self.names.value, start.inx())
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
                self.binds.get_val_mut(bind).unwrap().0 = true;
                if let Some(start) = comp.start {
                    *self.values.get_val_mut(start).unwrap() = true;
                }
                let start_s = if let Some(start) = comp.start {
                    format!("{}_{}", self.names.value, start.inx())
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
                    width.inx()
                )
                .unwrap();
            } else {
                write!(
                    s,
                    "{}={}({},{}_{});",
                    self.names.shl,
                    self.fn_names.usize_add,
                    self.names.shl,
                    self.names.width,
                    width.inx()
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
            // use copy_
            let sink = concat.comps[0].bind.unwrap();
            if from_buf {
                *self.binds.get_val_mut(sink).unwrap() = (true, true);
                return format!(
                    "let _ = {}({}_{},{});\n",
                    self.fn_names.copy_,
                    self.names.bind,
                    sink.inx(),
                    self.names.awi_ref,
                )
            } else {
                self.binds.get_val_mut(sink).unwrap().0 = true;
                return format!(
                    "let _ = {}({},{}_{});\n",
                    self.fn_names.copy_,
                    self.names.awi_ref,
                    self.names.bind,
                    sink.inx(),
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
                self.binds.get_val_mut(src).unwrap().0 = true;
                for i in 1..ast.cc.len() {
                    if let Some(sink) = ast.cc[i].comps[0].bind {
                        *self.binds.get_val_mut(sink).unwrap() = (true, true);
                        writeln!(
                            s,
                            "let _ = {}({}_{},{}_{});",
                            self.fn_names.copy_,
                            self.names.bind,
                            sink.inx(),
                            self.names.bind,
                            src.inx()
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
        for (p_cw, cw, used) in self.cw.iter() {
            if *used {
                let mut tmp = String::new();
                for (i, w) in cw.0.iter().enumerate() {
                    self.widths.get_val_mut(*w).unwrap().1 = true;
                    if i == 0 {
                        tmp = format!("{}_{}", self.names.width, w.inx());
                    } else {
                        tmp = format!(
                            "{}({},{}_{})",
                            self.fn_names.usize_add,
                            tmp,
                            self.names.width,
                            w.inx()
                        );
                    }
                }
                writeln!(s, "let {}_{}={};", self.names.cw, p_cw.inx(), tmp).unwrap();
            }
        }
        s
    }

    pub fn lower_widths(&mut self) -> String {
        let mut s = String::new();
        for (p_w, w, used) in self.widths.iter() {
            if used.1 {
                match w {
                    Width::Single(v) => {
                        *self.values.get_val_mut(*v).unwrap() = true;
                        writeln!(
                            s,
                            "let {}_{}={}_{};",
                            self.names.width,
                            p_w.inx(),
                            self.names.value,
                            v.inx()
                        )
                        .unwrap();
                    }
                    Width::Range(v0, v1) => {
                        *self.values.get_val_mut(*v0).unwrap() = true;
                        *self.values.get_val_mut(*v1).unwrap() = true;
                        writeln!(
                            s,
                            "let {}_{}={}({}_{},{}_{});",
                            self.names.width,
                            p_w.inx(),
                            self.fn_names.usize_sub,
                            self.names.value,
                            v1.inx(),
                            self.names.value,
                            v0.inx()
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
        for (p_v, v, used) in self.values.iter() {
            if *used {
                match v {
                    Value::Bitwidth(b) => {
                        self.binds.get_val_mut(*b).unwrap().0 = true;
                        writeln!(
                            s,
                            "let {}_{}={}({}({}_{}));",
                            self.names.value,
                            p_v.inx(),
                            self.fn_names.usize_cast,
                            self.fn_names.get_bw,
                            self.names.bind,
                            b.inx()
                        )
                        .unwrap();
                    }
                    Value::Usize(string) => {
                        writeln!(
                            s,
                            "let {}_{}={}({});",
                            self.names.value,
                            p_v.inx(),
                            self.fn_names.usize_cast,
                            string
                        )
                        .unwrap();
                    }
                }
            }
        }
        s
    }

    pub fn lower_bindings<F: FnMut(Awi) -> String>(
        &mut self,
        mut static_construction_fn: F,
    ) -> String {
        let mut s = String::new();
        for (p_b, bind, (used, mutable)) in self.binds.iter() {
            if *used {
                match bind {
                    Bind::Literal(ref awi) => {
                        writeln!(
                            s,
                            "let {}_{}:{}=&{};",
                            self.names.bind,
                            p_b.inx(),
                            self.fn_names.bits_ref,
                            (static_construction_fn)(Awi::from_bits(awi))
                        )
                        .unwrap();
                    }
                    Bind::Txt(ref chars) => {
                        let chars = chars_to_string(chars);
                        // note: we can't use extra braces in the bindings or else the E0716
                        // workaround doesn't work
                        if *mutable {
                            writeln!(
                                s,
                                "let {}_{}:{}=&mut {};",
                                self.names.bind,
                                p_b.inx(),
                                self.fn_names.mut_bits_ref,
                                chars
                            )
                            .unwrap();
                        } else {
                            writeln!(
                                s,
                                "let {}_{}:{}=&{};",
                                self.names.bind,
                                p_b.inx(),
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
