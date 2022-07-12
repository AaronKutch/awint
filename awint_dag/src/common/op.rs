use std::{
    cmp,
    fmt::{self, Debug},
    hash, mem,
    num::NonZeroUsize,
};

use awint_ext::ExtAwi;
use Op::*;

use crate::common::EvalError;

/// Mimicking operation
#[derive(Debug, Default, Clone, Hash, PartialEq, Eq)]
pub enum Op<T: fmt::Debug + Default + Clone + hash::Hash + PartialEq + cmp::Eq> {
    // A state used transiently by some algorithms, will cause errors if reached
    #[default]
    Invalid,

    // represents an unknown, arbitrary, or opaque-boxed source or sink (can have any number of
    // operands)
    Opaque(Vec<T>),

    // literal assign
    Literal(ExtAwi),

    // the bitwidth value
    //Bw,

    // note: we encourage common assembly code paths by putting the arrays first

    // These do not require the value of `self`, but do need the bitwidth. Note: only the overflow
    // variants actually need this in the current implementation of `awint_dag` that stores
    // bitwidth information in nodes, but I am making `Op` this way ahead of time, because of
    // future changes that may calculate bitwidth during evaluation.
    Resize([T; 2], NonZeroUsize),
    ZeroResize([T; 1], NonZeroUsize),
    SignResize([T; 1], NonZeroUsize),
    ZeroResizeOverflow([T; 1], NonZeroUsize),
    SignResizeOverflow([T; 1], NonZeroUsize),
    Lut([T; 2], NonZeroUsize),

    // these are special because although they take `&mut self`, the value of `self` is completely
    // overridden, so there is no dependency on `self.op()`.
    // I'm not sure what to do about dynamic `None` cases which would depend on `self`
    Copy([T; 1]),
    Funnel([T; 2]),
    UQuo([T; 2]),
    URem([T; 2]),
    IQuo([T; 2]),
    IRem([T; 2]),
    MulAdd([T; 3]),
    CinSum([T; 3]),
    UnsignedOverflow([T; 3]),
    SignedOverflow([T; 3]),

    // (&mut self)
    Not([T; 1]),
    Rev([T; 1]),
    Abs([T; 1]),

    // (&self) -> bool
    IsZero([T; 1]),
    IsUmax([T; 1]),
    IsImax([T; 1]),
    IsImin([T; 1]),
    IsUone([T; 1]),
    Lsb([T; 1]),
    Msb([T; 1]),

    // (&self) -> usize
    Lz([T; 1]),
    Tz([T; 1]),
    Sig([T; 1]),
    CountOnes([T; 1]),

    // (&mut self, rhs: &Self)
    Or([T; 2]),
    And([T; 2]),
    Xor([T; 2]),
    Shl([T; 2]),
    Lshr([T; 2]),
    Ashr([T; 2]),
    Rotl([T; 2]),
    Rotr([T; 2]),
    Add([T; 2]),
    Sub([T; 2]),
    Rsb([T; 2]),

    // (&self, rhs: &Self) -> Option<bool>
    Eq([T; 2]),
    Ne([T; 2]),
    Ult([T; 2]),
    Ule([T; 2]),
    Ilt([T; 2]),
    Ile([T; 2]),

    Inc([T; 2]),
    IncCout([T; 2]),
    Dec([T; 2]),
    DecCout([T; 2]),
    Neg([T; 2]),

    Get([T; 2]),
    Set([T; 3]),
    LutSet([T; 3]),
    // prevent all `Op<T>` from needing to be more than `[T;4]` in size
    Field(Box<[T; 5]>),
    FieldTo([T; 4]),
    FieldFrom([T; 4]),
    FieldWidth([T; 3]),
    FieldBit([T; 4]),
}

macro_rules! map1 {
    ($map:ident, $v:ident) => {{
        let mut res = [Default::default()];
        $map(&mut res, $v);
        res
    }};
}
macro_rules! map2 {
    ($map:ident, $v:ident) => {{
        let mut res = [Default::default(), Default::default()];
        $map(&mut res, $v);
        res
    }};
}
macro_rules! map3 {
    ($map:ident, $v:ident) => {{
        let mut res = [Default::default(), Default::default(), Default::default()];
        $map(&mut res, $v);
        res
    }};
}
macro_rules! map4 {
    ($map:ident, $v:ident) => {{
        let mut res = [
            Default::default(),
            Default::default(),
            Default::default(),
            Default::default(),
        ];
        $map(&mut res, $v);
        res
    }};
}

impl<T: fmt::Debug + Default + Clone + hash::Hash + PartialEq + cmp::Eq> Op<T> {
    /// This replaces `self` with `Invalid` and moves out literals without
    /// cloning them
    pub fn take(&mut self) -> Self {
        mem::take(self)
    }

    /// Returns if `self` is a `Literal`
    pub fn is_literal(&self) -> bool {
        matches!(self, Literal(_))
    }

    /// Returns the name of the operation
    pub fn operation_name(&self) -> &'static str {
        match *self {
            Invalid => "invalid",
            Opaque(_) => "opaque",
            Literal(_) => "literal",
            Resize(..) => "resize",
            ZeroResize(..) => "zero_resize",
            ZeroResizeOverflow(..) => "zero_reisze_overflow",
            SignResize(..) => "sign_resize",
            SignResizeOverflow(..) => "sign_resize_overflow",
            Lut(..) => "lut",
            Copy(_) => "copy",
            Funnel(_) => "funnel",
            UQuo(_) => "uquo",
            URem(_) => "urem",
            IQuo(_) => "iquo",
            IRem(_) => "irem",
            MulAdd(_) => "mul_add",
            CinSum(_) => "cin_sum",
            UnsignedOverflow(_) => "unsigned_overflow",
            SignedOverflow(_) => "signed_overflow",
            Not(_) => "not",
            Rev(_) => "rev",
            Abs(_) => "abs",
            IsZero(_) => "is_zero",
            IsUmax(_) => "is_umax",
            IsImax(_) => "is_imax",
            IsImin(_) => "is_imin",
            IsUone(_) => "is_uone",
            Lsb(_) => "lsb",
            Msb(_) => "msb",
            Lz(_) => "lz",
            Tz(_) => "tz",
            Sig(_) => "sig",
            CountOnes(_) => "count_ones",
            Or(_) => "or",
            And(_) => "and",
            Xor(_) => "xor",
            Shl(_) => "shl",
            Lshr(_) => "lshr",
            Ashr(_) => "ashr",
            Rotl(_) => "rotl",
            Rotr(_) => "rotr",
            Add(_) => "add",
            Sub(_) => "sub",
            Rsb(_) => "rsb",
            Eq(_) => "eq",
            Ne(_) => "ne",
            Ult(_) => "ult",
            Ule(_) => "ule",
            Ilt(_) => "ilt",
            Ile(_) => "ile",
            Inc(_) => "inc",
            IncCout(_) => "inc_cout",
            Dec(_) => "dec",
            DecCout(_) => "dec_cout",
            Neg(_) => "neg",
            Get(_) => "get",
            Set(_) => "set",
            LutSet(_) => "lut_set",
            Field(_) => "field",
            FieldTo(_) => "field_to",
            FieldFrom(_) => "field_from",
            FieldWidth(_) => "field_width",
            FieldBit(_) => "field_bit",
        }
    }

    /// Returns names of operands
    pub fn operand_names(&self) -> Vec<&'static str> {
        let mut v = vec![];
        // add common "lhs"
        match *self {
            Invalid | Opaque(_) | Literal(_) => (),

            Resize(..) => {
                v.push("x");
                v.push("extension");
            }
            Copy(_)
            | ZeroResize(..)
            | SignResize(..)
            | ZeroResizeOverflow(..)
            | SignResizeOverflow(..) => {
                v.push("x");
            }
            Lut(..) => {
                v.push("lut");
                v.push("inx")
            }
            Funnel(_) => {
                v.push("x");
                v.push("s");
            }
            UQuo(_) | URem(_) | IQuo(_) | IRem(_) => {
                v.push("duo");
                v.push("div");
            }
            MulAdd(_) => {
                v.push("add");
                v.push("lhs");
                v.push("rhs");
            }
            CinSum(_) | UnsignedOverflow(_) | SignedOverflow(_) => {
                v.push("cin");
                v.push("lhs");
                v.push("rhs");
            }

            Not(_) | Rev(_) | Abs(_) => v.push("x"),

            IsZero(_) | IsUmax(_) | IsImax(_) | IsImin(_) | IsUone(_) | Lsb(_) | Msb(_) => {
                v.push("x")
            }

            Lz(_) | Tz(_) | Sig(_) | CountOnes(_) => v.push("x"),

            Or(_) | And(_) | Xor(_) | Shl(_) | Lshr(_) | Ashr(_) | Rotl(_) | Rotr(_) | Add(_)
            | Sub(_) | Rsb(_) => {
                v.push("lhs");
                v.push("rhs")
            }

            Eq(_) | Ne(_) | Ult(_) | Ule(_) | Ilt(_) | Ile(_) => {
                v.push("lhs");
                v.push("rhs");
            }

            Inc(_) | IncCout(_) | Dec(_) | DecCout(_) => {
                v.push("x");
                v.push("cin");
            }
            Neg(_) => {
                v.push("x");
                v.push("neg");
            }
            Get(_) => {
                v.push("x");
                v.push("inx");
            }
            Set(_) => {
                v.push("x");
                v.push("inx");
                v.push("bit");
            }
            LutSet(_) => {
                v.push("lut");
                v.push("entry");
                v.push("inx");
            }
            ref op @ (Field(_) | FieldTo(_) | FieldFrom(_) | FieldWidth(_) | FieldBit(_)) => {
                v.push("lhs");
                if !matches!(op, FieldFrom(_) | FieldWidth(_)) {
                    v.push("to");
                }
                v.push("rhs");
                if !matches!(op, FieldTo(_) | FieldWidth(_)) {
                    v.push("from");
                }
                if !matches!(op, FieldBit(_)) {
                    v.push("width");
                }
            }
        }
        v
    }

    pub fn operands(&self) -> &[T] {
        match self {
            Invalid => &[],
            Opaque(v) => v,
            Literal(_) => &[],
            Resize(v, _) => v,
            ZeroResize(v, _) => v,
            SignResize(v, _) => v,
            ZeroResizeOverflow(v, _) => v,
            SignResizeOverflow(v, _) => v,
            Lut(v, _) => v,
            Copy(v) => v,
            Funnel(v) => v,
            UQuo(v) => v,
            URem(v) => v,
            IQuo(v) => v,
            IRem(v) => v,
            MulAdd(v) => v,
            CinSum(v) => v,
            UnsignedOverflow(v) => v,
            SignedOverflow(v) => v,
            Not(v) => v,
            Rev(v) => v,
            Abs(v) => v,
            IsZero(v) => v,
            IsUmax(v) => v,
            IsImax(v) => v,
            IsImin(v) => v,
            IsUone(v) => v,
            Lsb(v) => v,
            Msb(v) => v,
            Lz(v) => v,
            Tz(v) => v,
            Sig(v) => v,
            CountOnes(v) => v,
            Or(v) => v,
            And(v) => v,
            Xor(v) => v,
            Shl(v) => v,
            Lshr(v) => v,
            Ashr(v) => v,
            Rotl(v) => v,
            Rotr(v) => v,
            Add(v) => v,
            Sub(v) => v,
            Rsb(v) => v,
            Eq(v) => v,
            Ne(v) => v,
            Ult(v) => v,
            Ule(v) => v,
            Ilt(v) => v,
            Ile(v) => v,
            Inc(v) => v,
            IncCout(v) => v,
            Dec(v) => v,
            DecCout(v) => v,
            Neg(v) => v,
            Get(v) => v,
            Set(v) => v,
            LutSet(v) => v,
            Field(v) => v.as_ref(),
            FieldTo(v) => v,
            FieldFrom(v) => v,
            FieldWidth(v) => v,
            FieldBit(v) => v,
        }
    }

    pub fn operands_mut(&mut self) -> &mut [T] {
        match self {
            Invalid => &mut [],
            Opaque(v) => v,
            Literal(_) => &mut [],
            Resize(v, _) => v,
            ZeroResize(v, _) => v,
            SignResize(v, _) => v,
            ZeroResizeOverflow(v, _) => v,
            SignResizeOverflow(v, _) => v,
            Lut(v, _) => v,
            Copy(v) => v,
            Funnel(v) => v,
            UQuo(v) => v,
            URem(v) => v,
            IQuo(v) => v,
            IRem(v) => v,
            MulAdd(v) => v,
            CinSum(v) => v,
            UnsignedOverflow(v) => v,
            SignedOverflow(v) => v,
            Not(v) => v,
            Rev(v) => v,
            Abs(v) => v,
            IsZero(v) => v,
            IsUmax(v) => v,
            IsImax(v) => v,
            IsImin(v) => v,
            IsUone(v) => v,
            Lsb(v) => v,
            Msb(v) => v,
            Lz(v) => v,
            Tz(v) => v,
            Sig(v) => v,
            CountOnes(v) => v,
            Or(v) => v,
            And(v) => v,
            Xor(v) => v,
            Shl(v) => v,
            Lshr(v) => v,
            Ashr(v) => v,
            Rotl(v) => v,
            Rotr(v) => v,
            Add(v) => v,
            Sub(v) => v,
            Rsb(v) => v,
            Eq(v) => v,
            Ne(v) => v,
            Ult(v) => v,
            Ule(v) => v,
            Ilt(v) => v,
            Ile(v) => v,
            Inc(v) => v,
            IncCout(v) => v,
            Dec(v) => v,
            DecCout(v) => v,
            Neg(v) => v,
            Get(v) => v,
            Set(v) => v,
            LutSet(v) => v,
            Field(v) => v.as_mut(),
            FieldTo(v) => v,
            FieldFrom(v) => v,
            FieldWidth(v) => v,
            FieldBit(v) => v,
        }
    }

    pub fn num_operands(&self) -> usize {
        self.operands().len()
    }

    /// Some variants have a bitwidth field that can be mutated with this
    pub fn self_bitwidth_mut(&mut self) -> Option<&mut NonZeroUsize> {
        match self {
            Invalid => None,
            Opaque(_) => None,
            Literal(_) => None,
            Resize(_, w) => Some(w),
            ZeroResize(_, w) => Some(w),
            SignResize(_, w) => Some(w),
            ZeroResizeOverflow(_, w) => Some(w),
            SignResizeOverflow(_, w) => Some(w),
            Lut(_, w) => Some(w),
            Copy(_) => None,
            Funnel(_) => None,
            UQuo(_) => None,
            URem(_) => None,
            IQuo(_) => None,
            IRem(_) => None,
            MulAdd(_) => None,
            CinSum(_) => None,
            UnsignedOverflow(_) => None,
            SignedOverflow(_) => None,
            Not(_) => None,
            Rev(_) => None,
            Abs(_) => None,
            IsZero(_) => None,
            IsUmax(_) => None,
            IsImax(_) => None,
            IsImin(_) => None,
            IsUone(_) => None,
            Lsb(_) => None,
            Msb(_) => None,
            Lz(_) => None,
            Tz(_) => None,
            Sig(_) => None,
            CountOnes(_) => None,
            Or(_) => None,
            And(_) => None,
            Xor(_) => None,
            Shl(_) => None,
            Lshr(_) => None,
            Ashr(_) => None,
            Rotl(_) => None,
            Rotr(_) => None,
            Add(_) => None,
            Sub(_) => None,
            Rsb(_) => None,
            Eq(_) => None,
            Ne(_) => None,
            Ult(_) => None,
            Ule(_) => None,
            Ilt(_) => None,
            Ile(_) => None,
            Inc(_) => None,
            IncCout(_) => None,
            Dec(_) => None,
            DecCout(_) => None,
            Neg(_) => None,
            Get(_) => None,
            Set(_) => None,
            LutSet(_) => None,
            Field(_) => None,
            FieldTo(_) => None,
            FieldFrom(_) => None,
            FieldWidth(_) => None,
            FieldBit(_) => None,
        }
    }

    /// If `this` has no operands (including `Opaque`s with empty `Vec`s) then
    /// this translation succeeds.
    pub fn translate_root<U: fmt::Debug + Default + Clone + hash::Hash + PartialEq + cmp::Eq>(
        this: &Op<U>,
    ) -> Option<Self> {
        match this {
            Invalid => Some(Invalid),
            Opaque(v) => {
                if v.is_empty() {
                    Some(Opaque(vec![]))
                } else {
                    None
                }
            }
            Literal(lit) => Some(Literal(lit.clone())),
            _ => None,
        }
    }

    // this is structured this way to avoid excessive allocations after the initial
    // mimick stage
    pub fn translate<
        U: fmt::Debug + Default + Clone + hash::Hash + PartialEq + cmp::Eq,
        F: FnMut(&mut [T], &[U]),
    >(
        this: &Op<U>,
        map: F,
    ) -> Self {
        let mut m = map;
        match this {
            Invalid => Invalid,
            Opaque(v) => {
                let mut res = Opaque(vec![Default::default(); v.len()]);
                m(res.operands_mut(), this.operands());
                res
            }
            Literal(lit) => Literal(lit.clone()),
            Resize(v, w) => Resize(map2!(m, v), *w),
            ZeroResize(v, w) => ZeroResize(map1!(m, v), *w),
            SignResize(v, w) => SignResize(map1!(m, v), *w),
            ZeroResizeOverflow(v, w) => ZeroResizeOverflow(map1!(m, v), *w),
            SignResizeOverflow(v, w) => SignResizeOverflow(map1!(m, v), *w),
            Lut(v, w) => Lut(map2!(m, v), *w),
            Copy(v) => Copy(map1!(m, v)),
            Funnel(v) => Funnel(map2!(m, v)),
            UQuo(v) => UQuo(map2!(m, v)),
            URem(v) => URem(map2!(m, v)),
            IQuo(v) => IQuo(map2!(m, v)),
            IRem(v) => IRem(map2!(m, v)),
            MulAdd(v) => MulAdd(map3!(m, v)),
            CinSum(v) => CinSum(map3!(m, v)),
            UnsignedOverflow(v) => UnsignedOverflow(map3!(m, v)),
            SignedOverflow(v) => SignedOverflow(map3!(m, v)),
            Not(v) => Not(map1!(m, v)),
            Rev(v) => Rev(map1!(m, v)),
            Abs(v) => Abs(map1!(m, v)),
            IsZero(v) => IsZero(map1!(m, v)),
            IsUmax(v) => IsUmax(map1!(m, v)),
            IsImax(v) => IsImax(map1!(m, v)),
            IsImin(v) => IsImin(map1!(m, v)),
            IsUone(v) => IsUone(map1!(m, v)),
            Lsb(v) => Lsb(map1!(m, v)),
            Msb(v) => Msb(map1!(m, v)),
            Lz(v) => Lz(map1!(m, v)),
            Tz(v) => Tz(map1!(m, v)),
            Sig(v) => Sig(map1!(m, v)),
            CountOnes(v) => CountOnes(map1!(m, v)),
            Or(v) => Or(map2!(m, v)),
            And(v) => And(map2!(m, v)),
            Xor(v) => Xor(map2!(m, v)),
            Shl(v) => Shl(map2!(m, v)),
            Lshr(v) => Lshr(map2!(m, v)),
            Ashr(v) => Ashr(map2!(m, v)),
            Rotl(v) => Rotl(map2!(m, v)),
            Rotr(v) => Rotr(map2!(m, v)),
            Add(v) => Add(map2!(m, v)),
            Sub(v) => Sub(map2!(m, v)),
            Rsb(v) => Rsb(map2!(m, v)),
            Eq(v) => Eq(map2!(m, v)),
            Ne(v) => Ne(map2!(m, v)),
            Ult(v) => Ult(map2!(m, v)),
            Ule(v) => Ule(map2!(m, v)),
            Ilt(v) => Ilt(map2!(m, v)),
            Ile(v) => Ile(map2!(m, v)),
            Inc(v) => Inc(map2!(m, v)),
            IncCout(v) => IncCout(map2!(m, v)),
            Dec(v) => Dec(map2!(m, v)),
            DecCout(v) => DecCout(map2!(m, v)),
            Neg(v) => Neg(map2!(m, v)),
            Get(v) => Get(map2!(m, v)),
            Set(v) => Set(map3!(m, v)),
            LutSet(v) => LutSet(map3!(m, v)),
            Field(_) => {
                let mut res = Field(Box::new([
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    Default::default(),
                ]));
                m(res.operands_mut(), this.operands());
                res
            }
            FieldTo(v) => FieldTo(map4!(m, v)),
            FieldFrom(v) => FieldFrom(map4!(m, v)),
            FieldWidth(v) => FieldWidth(map3!(m, v)),
            FieldBit(v) => FieldBit(map4!(m, v)),
        }
    }
}

impl<T: fmt::Debug + Default + Clone + hash::Hash + PartialEq + cmp::Eq> Op<T> {
    // Checks validity of bitwidths. `self_bw` is bitwidth
    // of the `self` operand. Returns `true` if a bitwidth is zero or a mismatch
    // error has occured.
    pub fn check_bitwidths(&self, self_bw: usize) -> bool {
        if self_bw == 0 {
            return true
        }
        let bw = self_bw;
        match self {
            /*Literal(_) | Invalid(_) | Opaque(_) => false,

            Copy(_) | Not(_) | Rev(_) | Neg(_) | Abs(_) | UQuo(_) | URem(_) | IQuo(_) | IRem(_) | MulAdd(_) | Or(_) | And(_) | Xor(_)
            | Add(_) | Sub(_) | Rsb(_) => {
                let mut b = false;
                for x in v {
                    if *x != bw {
                        b = true;
                        break
                    }
                }
                b
            }

            Eq(_) | Ne(_) | Ult(_) | Ule(_) | Ilt(_) | Ile(_) => (bw != 1) || (v[0] != v[1]),

            Resize(_, nzbw) => (bw != nzbw.get()) || (v[0] != 1),

            ZeroResize(_, nzbw) | SignResize(_, nzbw) => bw != nzbw.get(),

            Lz(_) | Tz(_) | Sig(_) | CountOnes(_) => bw != BITS,

            Lsb(_)
            | Msb(_)
            | IsZero(_)
            | IsUmax(_)
            | IsImax(_)
            | IsImin(_)
            | IsUone(_)
            | ZeroResizeOverflow(_)
            | SignResizeOverflow(_) => bw != 1,

            IncCout(_) | DecCout(_) => (bw != 1) || (v[1] != 1),

            Shl(_) | Lshr(_) | Ashr(_) | Rotl(_) | Rotr(_) => (bw != v[0]) || (v[1] != BITS),

            CinSum(_) => (v[0] != 1) || (bw != v[1]) || (bw != v[2]),

            UnsignedOverflow(_) | SignedOverflow(_) => (bw != 1) || (v[0] != v[1]),

            Inc(_) | Dec(_) => (bw != v[0]) || (v[1] != 1),

            Lut(_, nzbw) => {
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
            Funnel(_) => (v[1] >= (BITS - 1)) || ((1usize << v[1]) != bw) || ((bw << 1) != v[0]),
            Get(_) => (bw != 1) || (v[1] != BITS),
            Set(_) => (bw != v[0]) || (v[1] != BITS) || (v[2] != 1),
            LutSet(_) => {
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
            Field(_) => (bw != v[0]) || (v[1] != BITS) || (v[3] != BITS) || (v[4] != BITS),
            FieldTo(_) => (bw != v[0]) || (v[1] != BITS) || (v[3] != BITS),
            FieldFrom(_) => (bw != v[0]) || (v[2] != BITS) || (v[3] != BITS),
            FieldWidth(_) => (bw != v[0]) || (v[2] != BITS),
            FieldBit(_) => (bw != v[0]) || (v[1] != BITS) || (v[3] != BITS),
            */
            _ => todo!(),
        }
    }

    // Checks that the values of operands are correct. Assumes that errors from
    // `expected_operands_len` and `check_bitwidths` have already been caught.
    // `self_bw` is the bitwidth of the `self` operand, and `ops` is a vector
    // of all the literal operands.
    /*pub fn check_values(&self, self_bw: NonZeroUsize, ops: &[ExtAwi]) -> bool {
        let v = ops;
        let bw = self_bw.get();
        match self {
            Literal(_)
            | Invalid(_)
            | Opaque(_)
            | Resize(_)
            | ZeroResize(_)
            | SignResize(_)
            | Copy(_)
            | Not(_)
            | Rev(_)
            | Abs(_)
            | IsZero(_)
            | IsUmax(_)
            | IsImax(_)
            | IsImin(_)
            | IsUone(_)
            | Lsb(_)
            | Msb(_)
            | Lz(_)
            | Tz(_)
            | Sig(_)
            | CountOnes(_)
            | ZeroResizeOverflow(_)
            | SignResizeOverflow(_)
            | Lut(_)
            | Funnel(_)
            | UQuo(_)
            | URem(_)
            | IQuo(_)
            | IRem(_)
            | MulAdd(_)
            | CinSum(_)
            | UnsignedOverflow(_)
            | SignedOverflow(_)
            | Or(_)
            | And(_)
            | Xor(_)
            | Add(_)
            | Sub(_)
            | Rsb(_)
            | Eq(_)
            | Ne(_)
            | Ult(_)
            | Ule(_)
            | Ilt(_)
            | Ile(_)
            | Inc(_)
            | IncCout(_)
            | Dec(_)
            | DecCout(_)
            | Neg(_)
            | LutSet(_) => false,

            Shl(_) | Lshr(_) | Ashr(_) | Rotl(_) | Rotr(_) => {
                let s = v[1].to_usize();
                s >= bw
            }
            Get(_) | Set(_) => v[1].to_usize() >= v[0].bw(),
            op @ (Field(_) | FieldTo(_) | FieldFrom(_) | FieldWidth(_) | FieldBit(_)) => {
                let width = if *op == FieldBit {
                    1
                } else {
                    v[v.len() - 1].to_usize()
                };
                let (to, x) = if matches!(op, FieldFrom(_) | FieldWidth(_)) {
                    (0, v[1].bw())
                } else {
                    (v[1].to_usize(), v[2].bw())
                };
                let from = if matches!(op, FieldTo(_) | FieldWidth(_)) {
                    0
                } else if *op == FieldFrom {
                    v[2].to_usize()
                } else {
                    v[3].to_usize()
                };
                (width > bw) || (width > x) || (to > (bw - width)) || (from > (x - width))
            }
        }
    }*/

    /// Evaluates the result of this operation, given `self_bw` and literal
    /// operands.
    pub fn eval(&self, self_bw: NonZeroUsize, ops: &[ExtAwi]) -> Result<ExtAwi, EvalError> {
        let mut eval_awi = ExtAwi::zero(self_bw);
        let e = eval_awi.const_as_mut();
        let mut tmp_awi = ExtAwi::zero(e.nzbw());
        let t = tmp_awi.const_as_mut();
        let mut tmp_awi1 = ExtAwi::zero(e.nzbw());
        let t1 = tmp_awi1.const_as_mut();
        let mut tmp_awi2 = ExtAwi::zero(e.nzbw());
        let t2 = tmp_awi2.const_as_mut();

        macro_rules! check_bw {
            ($lhs:expr, $rhs:expr) => {
                if $lhs != $rhs {
                    return Err(EvalError::WrongBitwidth)
                }
            };
        }

        let option = match self {
            Invalid => return Err(EvalError::Unevaluatable),
            Opaque(_) => return Err(EvalError::Unevaluatable),
            Literal(ref lit) => e.copy_assign(lit.const_as_ref()),
            /*Resize([a, b], w) => {
                check_bw!(*w, self_bw);
                e.resize_assign(a, b.to_bool());
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
            Copy(_) => e.copy_assign(v[0]),
            Lut(_) => e.lut(v[0], v[1]),
            Funnel(_) => e.funnel(v[0], v[1]),
            UQuo(_) => Bits::udivide(e, t, v[0], v[1]),
            URem(_) => Bits::udivide(t, e, v[0], v[1]),
            IQuo(_) => {
                t1.copy_assign(v[0])?;
                t2.copy_assign(v[1])?;
                Bits::idivide(e, t, t1, t2)
            }
            IRem(_) => {
                t1.copy_assign(v[0])?;
                t2.copy_assign(v[1])?;
                Bits::idivide(t, e, t1, t2)
            }
            MulAdd(_) => {
                e.copy_assign(v[0])?;
                e.mul_add_assign(v[1], v[2])
            }
            CinSum(_) => {
                if e.cin_sum_assign(v[0].to_bool(), v[1], v[2]).is_some() {
                    Some(())
                } else {
                    None
                }
            }
            UnsignedOverflow(_) => {
                if let Some((o, _)) = t.cin_sum_assign(v[0].to_bool(), v[1], v[2]) {
                    e.bool_assign(o);
                    Some(())
                } else {
                    None
                }
            }
            SignedOverflow(_) => {
                if let Some((_, o)) = t.cin_sum_assign(v[0].to_bool(), v[1], v[2]) {
                    e.bool_assign(o);
                    Some(())
                } else {
                    None
                }
            }
            Not(_) => {
                let r = e.copy_assign(v[0]);
                e.not_assign();
                r
            }
            Rev(_) => {
                let r = e.copy_assign(v[0]);
                e.rev_assign();
                r
            }
            Abs(_) => {
                let r = e.copy_assign(v[0]);
                e.abs_assign();
                r
            }
            IsZero(_) => {
                e.bool_assign(v[0].is_zero());
                Some(())
            }
            IsUmax(_) => {
                e.bool_assign(v[0].is_umax());
                Some(())
            }
            IsImax(_) => {
                e.bool_assign(v[0].is_imax());
                Some(())
            }
            IsImin(_) => {
                e.bool_assign(v[0].is_imin());
                Some(())
            }
            IsUone(_) => {
                e.bool_assign(v[0].is_uone());
                Some(())
            }
            Lsb(_) => {
                e.bool_assign(v[0].lsb());
                Some(())
            }
            Msb(_) => {
                e.bool_assign(v[0].msb());
                Some(())
            }
            Lz(_) => {
                e.usize_assign(v[0].lz());
                Some(())
            }
            Tz(_) => {
                e.usize_assign(v[0].tz());
                Some(())
            }
            Sig(_) => {
                e.usize_assign(v[0].sig());
                Some(())
            }
            CountOnes(_) => {
                e.usize_assign(v[0].count_ones());
                Some(())
            }
            Or(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.or_assign(v[1])
                } else {
                    None
                }
            }
            And(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.and_assign(v[1])
                } else {
                    None
                }
            }
            Xor(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.xor_assign(v[1])
                } else {
                    None
                }
            }

            Shl(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.shl_assign(v[1].to_usize())
                } else {
                    None
                }
            }
            Lshr(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.lshr_assign(v[1].to_usize())
                } else {
                    None
                }
            }
            Ashr(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.ashr_assign(v[1].to_usize())
                } else {
                    None
                }
            }
            Rotl(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.rotl_assign(v[1].to_usize())
                } else {
                    None
                }
            }
            Rotr(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.rotr_assign(v[1].to_usize())
                } else {
                    None
                }
            }
            Add(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.add_assign(v[1])
                } else {
                    None
                }
            }
            Sub(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.sub_assign(v[1])
                } else {
                    None
                }
            }
            Rsb(_) => {
                if let Some(()) = e.copy_assign(v[0]) {
                    e.rsb_assign(v[1])
                } else {
                    None
                }
            }
            Eq(_) => {
                if let Some(b) = v[0].const_eq(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ne(_) => {
                if let Some(b) = v[0].const_ne(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ult(_) => {
                if let Some(b) = v[0].ult(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ule(_) => {
                if let Some(b) = v[0].ule(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ilt(_) => {
                if let Some(b) = v[0].ilt(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Ile(_) => {
                if let Some(b) = v[0].ile(v[1]) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Inc(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.inc_assign(v[1].to_bool());
                    Some(())
                } else {
                    None
                }
            }
            Dec(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.dec_assign(v[1].to_bool());
                    Some(())
                } else {
                    None
                }
            }
            Neg(_) => {
                let r = e.copy_assign(v[0]);
                e.neg_assign(v[1].to_bool());
                r
            }
            IncCout(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.bool_assign(t.inc_assign(v[1].to_bool()));
                    Some(())
                } else {
                    None
                }
            }
            DecCout(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.bool_assign(t.dec_assign(v[1].to_bool()));
                    Some(())
                } else {
                    None
                }
            }
            ZeroResizeOverflow(_,nzbw) => {
                let mut tmp_awi3 = ExtAwi::zero(*nzbw);
                e.bool_assign(tmp_awi3.zero_resize_assign(v[0]));
                Some(())
            }
            SignResizeOverflow(_,nzbw) => {
                let mut tmp_awi3 = ExtAwi::zero(*nzbw);
                e.bool_assign(tmp_awi3.sign_resize_assign(v[0]));
                Some(())
            }
            Get(_) => {
                if let Some(b) = v[0].get(v[1].to_usize()) {
                    e.bool_assign(b);
                    Some(())
                } else {
                    None
                }
            }
            Set(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.set(v[1].to_usize(), v[2].to_bool())
                } else {
                    None
                }
            }
            LutSet(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.lut_set(v[1], v[2])
                } else {
                    None
                }
            }
            Field(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.field(v[1].to_usize(), v[2], v[3].to_usize(), v[4].to_usize())
                } else {
                    None
                }
            }
            FieldTo(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.field_to(v[1].to_usize(), v[2], v[3].to_usize())
                } else {
                    None
                }
            }
            FieldFrom(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.field_from(v[1], v[2].to_usize(), v[3].to_usize())
                } else {
                    None
                }
            }
            FieldWidth(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.field_width(v[1], v[2].to_usize())
                } else {
                    None
                }
            }
            FieldBit(_) => {
                if e.copy_assign(v[0]).is_some() {
                    e.field_bit(v[1].to_usize(), v[2], v[3].to_usize())
                } else {
                    None
                }
            }*/
            _ => todo!(),
        };
        if option.is_none() {
            Err(EvalError::WrongBitwidth)
        } else {
            Ok(eval_awi)
        }
    }
}
