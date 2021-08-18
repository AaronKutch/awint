use awint_core::Bits;
use awint_ext::ExtAwi;

/// Mimicking operation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Op {
    // literal assign
    Literal(ExtAwi),

    // A state used transiently by some algorithms, will cause errors if reached
    Invalid,

    // represents an unknown, arbitrary, or opaque-boxed source or sink (can have any number of
    // operands and dependents)
    Opaque,

    // the bitwidth value
    //Bw,

    // These do not require the value of `self`, but do need the bitwidth. Note: only the overflow
    // variants actually need this in the current implementation of `awint_dag` that stores
    // bitwidth information in nodes, but I am making `Op` this way ahead of time, because of
    // future changes that may calculate bitwidth during evaluation.
    Resize(NonZeroUsize),
    ZeroResize(NonZeroUsize),
    SignResize(NonZeroUsize),
    ZeroResizeOverflow(NonZeroUsize),
    SignResizeOverflow(NonZeroUsize),
    Lut(NonZeroUsize),

    // these are special because although they take `&mut self`, the value of `self` is completely
    // overridden, so there is no dependency on `self.op()`.
    // I'm not sure what to do about dynamic `None` cases which would depend on `self`
    Copy,
    Funnel,
    UQuo,
    URem,
    IQuo,
    IRem,
    MulAdd,
    CinSum,
    UnsignedOverflow,
    SignedOverflow,

    // (&mut self)
    Not,
    Rev,
    Neg,
    Abs,

    // (&self) -> bool
    IsZero,
    IsUmax,
    IsImax,
    IsImin,
    IsUone,
    Lsb,
    Msb,

    // (&self) -> usize
    Lz,
    Tz,
    CountOnes,

    // (&mut self, rhs: &Self)
    Or,
    And,
    Xor,
    Shl,
    Lshr,
    Ashr,
    Rotl,
    Rotr,
    Add,
    Sub,
    Rsb,

    // (&self, rhs: &Self) -> Option<bool>
    Eq,
    Ne,
    Ult,
    Ule,
    Ilt,
    Ile,

    Inc,
    Dec,
    IncCout,
    DecCout,

    LutSet,
    Field,
}

use std::num::NonZeroUsize;

use awint_internals::BITS;
use Op::*;

impl Op {
    /// Returns the name of the operation
    pub fn operation_name(&self) -> &'static str {
        match *self {
            Literal(_) => "literal",
            Invalid => "invalid",
            Opaque => "opaque",
            Resize(_) => "resize",
            ZeroResize(_) => "zero_resize",
            ZeroResizeOverflow(_) => "zero_reisze_overflow",
            SignResize(_) => "sign_resize",
            SignResizeOverflow(_) => "sign_resize_overflow",
            Copy => "copy",
            Lut(_) => "lut",
            Funnel => "funnel",
            UQuo => "uquo",
            URem => "urem",
            IQuo => "iquo",
            IRem => "irem",
            MulAdd => "mul_add",
            CinSum => "cin_sum",
            UnsignedOverflow => "unsigned_overflow",
            SignedOverflow => "signed_overflow",
            Not => "not",
            Rev => "rev",
            Neg => "neg",
            Abs => "abs",
            IsZero => "is_zero",
            IsUmax => "is_umax",
            IsImax => "is_imax",
            IsImin => "is_imin",
            IsUone => "is_uone",
            Lsb => "lsb",
            Msb => "msb",
            Lz => "lz",
            Tz => "tz",
            CountOnes => "count_ones",
            Or => "or",
            And => "and",
            Xor => "xor",
            Shl => "shl",
            Lshr => "lshr",
            Ashr => "ashr",
            Rotl => "rotl",
            Rotr => "rotr",
            Add => "add",
            Sub => "sub",
            Rsb => "rsb",
            Eq => "eq",
            Ne => "ne",
            Ult => "ult",
            Ule => "ule",
            Ilt => "ilt",
            Ile => "ile",
            Inc => "inc",
            IncCout => "inc_cout",
            Dec => "dec",
            DecCout => "dec_cout",
            LutSet => "lut_set",
            Field => "field",
        }
    }

    /// Returns names of operands
    pub fn operand_names(&self) -> Vec<&'static str> {
        let mut v = vec![];
        // add common "lhs"
        match *self {
            Literal(_) | Invalid | Opaque => (),

            Resize(_) => {
                v.push("x");
                v.push("extension");
            }
            Copy
            | ZeroResize(_)
            | SignResize(_)
            | ZeroResizeOverflow(_)
            | SignResizeOverflow(_) => {
                v.push("x");
            }
            Lut(_) => {
                v.push("lut");
                v.push("inx")
            }
            Funnel => {
                v.push("x");
                v.push("s");
            }
            UQuo | URem | IQuo | IRem => {
                v.push("duo");
                v.push("div");
            }
            MulAdd => {
                v.push("lhs");
                v.push("rhs");
            }
            CinSum | UnsignedOverflow | SignedOverflow => {
                v.push("cin");
                v.push("lhs");
                v.push("rhs");
            }

            Not | Rev | Neg | Abs => v.push("x"),

            IsZero | IsUmax | IsImax | IsImin | IsUone | Lsb | Msb => v.push("x"),

            Lz | Tz | CountOnes => v.push("x"),

            Or | And | Xor | Shl | Lshr | Ashr | Rotl | Rotr | Add | Sub | Rsb => {
                v.push("lhs");
                v.push("rhs")
            }

            Eq | Ne | Ult | Ule | Ilt | Ile => {
                v.push("lhs");
                v.push("rhs");
            }

            Inc | Dec | IncCout | DecCout => {
                v.push("x");
                v.push("cin");
            }
            LutSet => {
                v.push("lut");
                v.push("entry");
                v.push("inx");
            }
            Field => {
                v.push("lhs");
                v.push("to");
                v.push("rhs");
                v.push("from");
                v.push("width");
            }
        }
        v
    }

    /// Returns the expected number of operands for the given operation. `None`
    /// is returned if there can be any number of operands
    pub fn operands_len(&self) -> Option<usize> {
        Some(match self {
            Invalid | Opaque => return None,
            Literal(_) => 0,

            ZeroResize(_)
            | SignResize(_)
            | ZeroResizeOverflow(_)
            | SignResizeOverflow(_)
            | Copy
            | Not
            | Rev
            | Neg
            | Abs
            | IsZero
            | IsUmax
            | IsImax
            | IsImin
            | IsUone
            | Lsb
            | Msb
            | Lz
            | Tz
            | CountOnes => 1,

            Resize(_) | Lut(_) | Funnel | UQuo | URem | IQuo | IRem | MulAdd | CinSum
            | UnsignedOverflow | SignedOverflow | Or | And | Xor | Shl | Lshr | Ashr | Rotl
            | Rotr | Add | Sub | Rsb | Eq | Ne | Ult | Ule | Ilt | Ile | Inc | IncCout | Dec
            | DecCout => 2,

            LutSet => 3,
            Field => 5,
        })
    }

    /// Checks validity of bitwidths. Assumes that errors from
    /// `expected_operands_len` have already been caught. `self_bw` is bitwidth
    /// of the `self` operand, and `v` is a vector of the bitwidths of the rest
    /// of the operands. Returns `true` if a bitwidth mismatch error has
    /// occured.
    pub fn check_bitwidths(&self, self_bw: NonZeroUsize, v: &[usize]) -> bool {
        let bw = self_bw.get();
        match self {
            Literal(_) | Invalid | Opaque => false,

            Copy | Not | Rev | Neg | Abs | UQuo | URem | IQuo | IRem | MulAdd | Or | And | Xor
            | Add | Sub | Rsb => {
                let mut b = false;
                for x in v {
                    if *x != bw {
                        b = true;
                        break
                    }
                }
                b
            }

            Eq | Ne | Ult | Ule | Ilt | Ile => (bw != 1) || (v[0] != v[1]),

            Resize(nzbw) => (bw != nzbw.get()) || (v[0] != 1),

            ZeroResize(nzbw) | SignResize(nzbw) => bw != nzbw.get(),

            Lz | Tz | CountOnes => bw != BITS,

            Lsb
            | Msb
            | IsZero
            | IsUmax
            | IsImax
            | IsImin
            | IsUone
            | ZeroResizeOverflow(_)
            | SignResizeOverflow(_) => bw != 1,

            IncCout | DecCout => (bw != 1) || (v[1] != 1),

            Shl | Lshr | Ashr | Rotl | Rotr => (bw != v[0]) || (v[1] != BITS),

            CinSum => (v[0] != 1) || (bw != v[1]) || (bw != v[2]),

            UnsignedOverflow | SignedOverflow => (bw != 1) || (v[0] != v[1]),

            Inc | Dec => (bw != v[0]) || (v[1] != 1),

            Lut(nzbw) => {
                (bw != nzbw.get())
                    || if v[1] < BITS {
                        if let Some(lut_len) = (1usize << v[1]).checked_mul(bw) {
                            lut_len != v[0]
                        } else {
                            true
                        }
                    } else {
                        true
                    }
            }
            Funnel => (v[1] >= (BITS - 1)) || ((1usize << v[1]) != bw) || ((bw << 1) != v[0]),
            LutSet => {
                (bw != v[0])
                    || if v[2] < BITS {
                        if let Some(lut_len) = (1usize << v[2]).checked_mul(v[1]) {
                            lut_len != bw
                        } else {
                            true
                        }
                    } else {
                        true
                    }
            }
            Field => (bw != v[0]) || (v[1] != BITS) || (v[3] != BITS) || (v[4] != BITS),
        }
    }

    /// Checks that the values of operands are correct. Assumes that errors from
    /// `expected_operands_len` and `check_bitwidths` have already been caught.
    /// `self_bw` is the bitwidth of the `self` operand, and `ops` is a vector
    /// of all the literal operands.
    pub fn check_values(&self, self_bw: NonZeroUsize, ops: &[ExtAwi]) -> bool {
        let v = ops;
        let bw = self_bw.get();
        match self {
            Literal(_)
            | Invalid
            | Opaque
            | Resize(_)
            | ZeroResize(_)
            | SignResize(_)
            | Copy
            | Not
            | Rev
            | Neg
            | Abs
            | IsZero
            | IsUmax
            | IsImax
            | IsImin
            | IsUone
            | Lsb
            | Msb
            | Lz
            | Tz
            | CountOnes
            | ZeroResizeOverflow(_)
            | SignResizeOverflow(_)
            | Lut(_)
            | Funnel
            | UQuo
            | URem
            | IQuo
            | IRem
            | MulAdd
            | CinSum
            | UnsignedOverflow
            | SignedOverflow
            | Or
            | And
            | Xor
            | Add
            | Sub
            | Rsb
            | Eq
            | Ne
            | Ult
            | Ule
            | Ilt
            | Ile
            | Inc
            | IncCout
            | Dec
            | DecCout
            | LutSet => false,

            Shl | Lshr | Ashr | Rotl | Rotr => {
                let s = v[1].const_as_ref().to_usize();
                s >= bw
            }
            Field => {
                let to = v[1].const_as_ref().to_usize();
                let from = v[3].const_as_ref().to_usize();
                let width = v[4].const_as_ref().to_usize();
                (width > bw)
                    || (width > v[2].bw())
                    || (to > (bw - width))
                    || (from > (v[2].bw() - width))
            }
        }
    }

    /// Evaluates the result of this operation, given `self_bw` and literal
    /// operands.
    pub fn eval(&self, self_bw: NonZeroUsize, ops: &[ExtAwi]) -> Option<ExtAwi> {
        let mut eval_awi = ExtAwi::zero(self_bw);
        let e = eval_awi.const_as_mut();
        let mut tmp_awi = ExtAwi::zero(e.nzbw());
        let t = tmp_awi.const_as_mut();
        let mut tmp_awi1 = ExtAwi::zero(e.nzbw());
        let t1 = tmp_awi1.const_as_mut();
        let mut tmp_awi2 = ExtAwi::zero(e.nzbw());
        let t2 = tmp_awi2.const_as_mut();

        let mut v: Vec<&Bits> = vec![];
        for op in ops {
            v.push(op.const_as_ref());
        }
        let option = match self {
            Literal(ref lit) => e.copy_assign(lit.const_as_ref()),
            Invalid => None,
            Opaque => None,
            Resize(_) => {
                e.resize_assign(v[0], v[1].to_bool());
                Some(())
            }
            ZeroResize(_) => {
                e.zero_resize_assign(v[0]);
                Some(())
            }
            SignResize(_) => {
                e.sign_resize_assign(v[0]);
                Some(())
            }
            Copy => e.copy_assign(v[0]),
            Lut(_) => e.lut(v[0], v[1]),
            Funnel => e.funnel(v[0], v[1]),
            UQuo => Bits::udivide(e, t, v[0], v[1]),
            URem => Bits::udivide(t, e, v[0], v[1]),
            IQuo => {
                t1.copy_assign(v[0])?;
                t2.copy_assign(v[1])?;
                Bits::idivide(e, t, t1, t2)
            }
            IRem => {
                t1.copy_assign(v[0])?;
                t2.copy_assign(v[1])?;
                Bits::idivide(t, e, t1, t2)
            }
            MulAdd => e.mul_add_triop(v[0], v[1]),
            CinSum => {
                if e.cin_sum_triop(v[0].to_bool(), v[1], v[2]).is_some() {
                    Some(())
                } else {
                    None
                }
            }
            UnsignedOverflow => {
                if let Some((o, _)) = t.cin_sum_triop(v[0].to_bool(), v[1], v[2]) {
                    e.bool_assign(o);
                    Some(())
                } else {
                    None
                }
            }
            SignedOverflow => {
                if let Some((_, o)) = t.cin_sum_triop(v[0].to_bool(), v[1], v[2]) {
                    e.bool_assign(o);
                    Some(())
                } else {
                    None
                }
            }
            Not => {
                let r = e.copy_assign(v[0]);
                e.not_assign();
                r
            }
            Rev => {
                let r = e.copy_assign(v[0]);
                e.rev_assign();
                r
            }
            Neg => {
                let r = e.copy_assign(v[0]);
                e.neg_assign();
                r
            }
            Abs => {
                let r = e.copy_assign(v[0]);
                e.abs_assign();
                r
            }
            IsZero => {
                e.bool_assign(v[0].is_zero());
                Some(())
            }
            IsUmax => {
                e.bool_assign(v[0].is_umax());
                Some(())
            }
            IsImax => {
                e.bool_assign(v[0].is_imax());
                Some(())
            }
            IsImin => {
                e.bool_assign(v[0].is_imin());
                Some(())
            }
            IsUone => {
                e.bool_assign(v[0].is_uone());
                Some(())
            }
            Lsb => {
                e.bool_assign(v[0].lsb());
                Some(())
            }
            Msb => {
                e.bool_assign(v[0].msb());
                Some(())
            }
            Lz => {
                e.usize_assign(v[0].lz());
                Some(())
            }
            Tz => {
                e.usize_assign(v[0].tz());
                Some(())
            }
            CountOnes => {
                e.usize_assign(v[0].count_ones());
                Some(())
            }
            Or => {
                if e.copy_assign(v[0]).is_some() {
                    e.or_assign(v[1])
                } else {
                    None
                }
            }
            And => {
                if e.copy_assign(v[0]).is_some() {
                    e.and_assign(v[1])
                } else {
                    None
                }
            }
            Xor => {
                if e.copy_assign(v[0]).is_some() {
                    e.xor_assign(v[1])
                } else {
                    None
                }
            }

            Shl => {
                if e.copy_assign(v[0]).is_some() {
                    e.shl_assign(v[1].to_usize())
                } else {
                    None
                }
            }
            Lshr => {
                if e.copy_assign(v[0]).is_some() {
                    e.lshr_assign(v[1].to_usize())
                } else {
                    None
                }
            }
            Ashr => {
                if e.copy_assign(v[0]).is_some() {
                    e.ashr_assign(v[1].to_usize())
                } else {
                    None
                }
            }
            Rotl => {
                if e.copy_assign(v[0]).is_some() {
                    e.rotl_assign(v[1].to_usize())
                } else {
                    None
                }
            }
            Rotr => {
                if e.copy_assign(v[0]).is_some() {
                    e.rotr_assign(v[1].to_usize())
                } else {
                    None
                }
            }
            Add => {
                if e.copy_assign(v[0]).is_some() {
                    e.add_assign(v[1])
                } else {
                    None
                }
            }
            Sub => {
                if e.copy_assign(v[0]).is_some() {
                    e.sub_assign(v[1])
                } else {
                    None
                }
            }
            Rsb => {
                if let Some(()) = e.copy_assign(v[0]) {
                    e.rsb_assign(v[1])
                } else {
                    None
                }
            }
            Eq => {
                if let Some(b) = v[0].const_eq(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ne => {
                if let Some(b) = v[0].const_ne(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ult => {
                if let Some(b) = v[0].ult(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ule => {
                if let Some(b) = v[0].ule(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ilt => {
                if let Some(b) = v[0].ilt(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ile => {
                if let Some(b) = v[0].ile(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Inc => {
                if e.copy_assign(v[0]).is_some() {
                    e.inc_assign(v[1].to_bool());
                    Some(())
                } else {
                    None
                }
            }
            Dec => {
                if e.copy_assign(v[0]).is_some() {
                    e.dec_assign(v[1].to_bool());
                    Some(())
                } else {
                    None
                }
            }
            IncCout => {
                if e.copy_assign(v[0]).is_some() {
                    e.bool_assign(t.inc_assign(v[1].to_bool()));
                    Some(())
                } else {
                    None
                }
            }
            DecCout => {
                if e.copy_assign(v[0]).is_some() {
                    e.bool_assign(t.dec_assign(v[1].to_bool()));
                    Some(())
                } else {
                    None
                }
            }
            ZeroResizeOverflow(nzbw) => {
                let mut tmp_awi3 = ExtAwi::zero(*nzbw);
                e.bool_assign(tmp_awi3[..].zero_resize_assign(v[0]));
                Some(())
            }
            SignResizeOverflow(nzbw) => {
                let mut tmp_awi3 = ExtAwi::zero(*nzbw);
                e.bool_assign(tmp_awi3[..].sign_resize_assign(v[0]));
                Some(())
            }
            LutSet => {
                if e.copy_assign(v[0]).is_some() {
                    e.lut_set(v[1], v[2])
                } else {
                    None
                }
            }
            Field => {
                if e.copy_assign(v[0]).is_some() {
                    e.field(v[1].to_usize(), v[2], v[3].to_usize(), v[4].to_usize())
                } else {
                    None
                }
            }
        };
        if option.is_none() {
            None
        } else {
            Some(eval_awi)
        }
    }
}
