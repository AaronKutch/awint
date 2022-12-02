use std::{
    cmp,
    fmt::{self, Debug},
    hash, mem,
    num::NonZeroUsize,
};

use awint_ext::ExtAwi;
use Op::*;

/// A mimicking `Op`eration
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

    // Static versions of `Lut`, `Get`, and `Set`
    StaticLut([T; 1], ExtAwi),
    StaticGet([T; 1], usize),
    StaticSet([T; 2], usize),

    // in the future we may try to do some kind of dynamic bitwidth
    //Bw,

    // note: we encourage common assembly code paths by putting the arrays first

    // These functions are special because they need self width or downstream width to operate. In
    // earlier versions these all had fields for size, but only the overflow variants actually
    // need it because their self width is a single bit and they are effectively parameterized
    Resize([T; 2]),
    ZeroResize([T; 1]),
    SignResize([T; 1]),
    ZeroResizeOverflow([T; 1], NonZeroUsize),
    SignResizeOverflow([T; 1], NonZeroUsize),
    Lut([T; 2]),

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
    Mux([T; 3]),
    LutSet([T; 3]),
    Field([T; 5]),
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

    /// Returns if `self` is an `Opaque`
    pub fn is_opaque(&self) -> bool {
        matches!(self, Opaque(_))
    }

    /// Returns if `self` is an `Invalid`
    pub fn is_invalid(&self) -> bool {
        matches!(self, Invalid)
    }

    /// Returns the name of the operation
    pub fn operation_name(&self) -> &'static str {
        match *self {
            Invalid => "invalid",
            Opaque(_) => "opaque",
            Literal(_) => "literal",
            StaticLut(..) => "static_lut",
            StaticGet(..) => "static_get",
            StaticSet(..) => "static_set",
            Resize(..) => "resize",
            ZeroResize(..) => "zero_resize",
            ZeroResizeOverflow(..) => "zero_reisze_overflow",
            SignResize(..) => "sign_resize",
            SignResizeOverflow(..) => "sign_resize_overflow",
            Lut(..) => "lut",
            Copy(_) => "copy",
            Funnel(_) => "funnel_",
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
            Mux(_) => "mux",
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
            StaticLut(..) => v.push("inx"),
            StaticGet(..) => v.push("x"),
            StaticSet(..) => {
                v.push("x");
                v.push("b")
            }

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
            Mux(_) => {
                v.push("x0");
                v.push("x1");
                v.push("b");
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
            StaticLut(v, _) => v,
            StaticGet(v, _) => v,
            StaticSet(v, _) => v,
            Resize(v) => v,
            ZeroResize(v) => v,
            SignResize(v) => v,
            ZeroResizeOverflow(v, _) => v,
            SignResizeOverflow(v, _) => v,
            Lut(v) => v,
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
            Mux(v) => v,
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
            StaticLut(v, _) => v,
            StaticGet(v, _) => v,
            StaticSet(v, _) => v,
            Resize(v) => v,
            ZeroResize(v) => v,
            SignResize(v) => v,
            ZeroResizeOverflow(v, _) => v,
            SignResizeOverflow(v, _) => v,
            Lut(v) => v,
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
            Mux(v) => v,
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
            StaticLut(v, table) => StaticLut(map1!(m, v), table.clone()),
            StaticGet(v, inx) => StaticGet(map1!(m, v), *inx),
            StaticSet(v, inx) => StaticSet(map2!(m, v), *inx),
            Resize(v) => Resize(map2!(m, v)),
            ZeroResize(v) => ZeroResize(map1!(m, v)),
            SignResize(v) => SignResize(map1!(m, v)),
            ZeroResizeOverflow(v, w) => ZeroResizeOverflow(map1!(m, v), *w),
            SignResizeOverflow(v, w) => SignResizeOverflow(map1!(m, v), *w),
            Lut(v) => Lut(map2!(m, v)),
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
            Mux(v) => Mux(map3!(m, v)),
            LutSet(v) => LutSet(map3!(m, v)),
            Field(_) => {
                let mut res = Field([
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    Default::default(),
                    Default::default(),
                ]);
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
