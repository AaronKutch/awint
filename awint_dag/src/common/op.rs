use std::{fmt::Debug, mem, num::NonZeroUsize};

use awint_ext::Awi;
use smallvec::{smallvec, SmallVec};
use thin_vec::ThinVec;
use Op::*;

use crate::DummyDefault;

#[derive(Debug, Default, Clone)]
pub struct ConcatType<T: Debug + DummyDefault + Clone> {
    v: SmallVec<[T; 4]>,
}

impl<T: Debug + DummyDefault + Clone> ConcatType<T> {
    /// Use only `smallvec![...]` to construct the argument for this. Panics if
    /// `v.is_empty()`.
    pub fn from_smallvec(v: SmallVec<[T; 4]>) -> Self {
        assert!(!v.is_empty());
        Self { v }
    }

    pub fn as_slice(&self) -> &[T] {
        self.v.as_slice()
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.v.as_mut_slice()
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.v.len()
    }
}

#[derive(Debug, Default, Clone)]
pub struct ConcatFieldsType<T: Debug + DummyDefault + Clone> {
    // needs to be separate because of the function requiring `&[T]` references
    v_t: ThinVec<T>,
    v_i: ThinVec<(usize, NonZeroUsize)>,
}

impl<T: Debug + DummyDefault + Clone> ConcatFieldsType<T> {
    /// Panics if `v.is_empty()` or the third element is zero.
    pub fn from_iter<I: IntoIterator<Item = (T, usize, NonZeroUsize)>>(
        capacity: usize,
        i: I,
    ) -> Self {
        let mut res = Self {
            v_t: ThinVec::with_capacity(capacity),
            v_i: ThinVec::with_capacity(capacity),
        };
        for item in i.into_iter() {
            res.v_t.push(item.0);
            res.v_i.push((item.1, item.2));
        }
        assert!(!res.v_t.is_empty());
        res
    }

    /// Adds another field for `self`
    pub fn push(&mut self, t: T, from: usize, width: NonZeroUsize) {
        self.v_t.push(t);
        self.v_i.push((from, width));
    }

    /// This is in order of source `T`, `from`, then `width`
    pub fn t_as_slice(&self) -> &[T] {
        self.v_t.as_slice()
    }

    pub fn t_as_mut_slice(&mut self) -> &mut [T] {
        self.v_t.as_mut_slice()
    }

    pub fn field_as_slice(&self) -> &[(usize, NonZeroUsize)] {
        self.v_i.as_slice()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&T, &(usize, NonZeroUsize))> {
        self.v_t.iter().zip(self.v_i.iter())
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.v_t.len()
    }
}

/// A mimicking `Op`eration
#[derive(Debug, Default, Clone)]
pub enum Op<T: Debug + DummyDefault + Clone> {
    // A state used transiently by some algorithms, will cause errors if reached
    #[default]
    Invalid,

    // represents an unknown, arbitrary, or opaque-boxed source or sink (can have any number of
    // operands)
    Opaque(SmallVec<[T; 2]>, Option<&'static str>),

    // literal assign
    Literal(Awi),

    // Assertion that a single bit is true
    Assert([T; 1]),

    // In previous versions of `awint_dag` we used to use networks of get and set operations which
    // turned out to be horribly inefficient. `Concat` just concats wholesale, and `ConcatFields`
    // concats fields from several sources together.
    Concat(ConcatType<T>),
    ConcatFields(ConcatFieldsType<T>),

    // Static version of `Lut`
    StaticLut([T; 1], Awi),

    // in the future we may try to do some kind of dynamic bitwidth
    //Bw,

    // note: we encourage common assembly code paths by putting the arrays first

    // These functions are special because they need self width or upstream width to operate. In
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
    ArbMulAdd([T; 3]),
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
        let mut res = [DummyDefault::default()];
        $map(&mut res, $v);
        res
    }};
}
macro_rules! map2 {
    ($map:ident, $v:ident) => {{
        let mut res = [DummyDefault::default(), DummyDefault::default()];
        $map(&mut res, $v);
        res
    }};
}
macro_rules! map3 {
    ($map:ident, $v:ident) => {{
        let mut res = [
            DummyDefault::default(),
            DummyDefault::default(),
            DummyDefault::default(),
        ];
        $map(&mut res, $v);
        res
    }};
}
macro_rules! map4 {
    ($map:ident, $v:ident) => {{
        let mut res = [
            DummyDefault::default(),
            DummyDefault::default(),
            DummyDefault::default(),
            DummyDefault::default(),
        ];
        $map(&mut res, $v);
        res
    }};
}

impl<T: Debug + DummyDefault + Clone> Op<T> {
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
        matches!(self, Opaque(_, _))
    }

    /// Returns if `self` is an `Invalid`
    pub fn is_invalid(&self) -> bool {
        matches!(self, Invalid)
    }

    /// Returns the name of the operation
    pub fn operation_name(&self) -> &'static str {
        match *self {
            Invalid => "invalid",
            Opaque(_, name) => name.unwrap_or("opaque"),
            Literal(_) => "literal",
            Assert(_) => "assert",
            Concat(_) => "concat",
            ConcatFields(_) => "concat_fields",
            StaticLut(..) => "static_lut",
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
            ArbMulAdd(_) => "mul_add",
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
            Invalid | Opaque(..) | Literal(_) => (),
            Assert(_) => v.push("b"),
            Concat(ref concat) => {
                v = vec!["c"; concat.len()];
            }
            ConcatFields(ref concat) => {
                v = vec!["c"; concat.len()];
            }
            StaticLut(..) => v.push("inx"),

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
            ArbMulAdd(_) => {
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

    /// Returns the `T` operands as a slice
    pub fn operands(&self) -> &[T] {
        match self {
            Invalid => &[],
            Opaque(v, _) => v,
            Literal(_) => &[],
            Assert(v) => v,
            Concat(concat) => concat.as_slice(),
            ConcatFields(concat) => concat.t_as_slice(),
            StaticLut(v, _) => v,
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
            ArbMulAdd(v) => v,
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

    /// Returns the `T` operands as a mutable slice
    pub fn operands_mut(&mut self) -> &mut [T] {
        match self {
            Invalid => &mut [],
            Opaque(v, _) => v,
            Literal(_) => &mut [],
            Assert(v) => v,
            Concat(concat) => concat.as_mut_slice(),
            ConcatFields(concat) => concat.t_as_mut_slice(),
            StaticLut(v, _) => v,
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
            ArbMulAdd(v) => v,
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

    /// Returns the number of operands
    pub fn operands_len(&self) -> usize {
        self.operands().len()
    }

    /// Translates an `Op<U>` into an `Op<T>`. Returns an error if `this` has a
    /// nonzero number of operands (`Opaque`s with empty `Vec`s succeed).
    pub fn translate_root<U: Debug + DummyDefault + Clone>(this: &Op<U>) -> Option<Self> {
        match this {
            Invalid => Some(Invalid),
            Opaque(v, name) => {
                if v.is_empty() {
                    Some(Opaque(smallvec![], *name))
                } else {
                    None
                }
            }
            Literal(lit) => Some(Literal(lit.clone())),
            _ => None,
        }
    }

    /// Translates an `Op<U>` into an `Op<T>`. It starts with each `T`
    /// initialized to the result of `DummyDefault`, then `map` is given
    /// a mutable reference to it and a reference to the corresponding `U`.
    pub fn translate<U: Debug + DummyDefault + Clone, F: FnMut(&mut [T], &[U])>(
        this: &Op<U>,
        map: F,
    ) -> Self {
        // this is structured this way to avoid excessive allocations after the initial
        // mimick stage
        let mut m = map;
        match this {
            Invalid => Invalid,
            Opaque(v, name) => {
                let mut res_v = smallvec![DummyDefault::default(); v.len()];
                m(res_v.as_mut_slice(), this.operands());
                Opaque(res_v, *name)
            }
            Literal(lit) => Literal(lit.clone()),
            Assert(v) => Assert(map1!(m, v)),
            Concat(ref concat) => {
                let mut res_concat =
                    ConcatType::from_smallvec(smallvec![DummyDefault::default(); concat.len()]);
                m(res_concat.as_mut_slice(), this.operands());
                Concat(res_concat)
            }
            ConcatFields(ref concat) => {
                let mut res_concat = ConcatFieldsType {
                    v_t: ThinVec::with_capacity(concat.len()),
                    v_i: ThinVec::with_capacity(concat.len()),
                };
                for (from, width) in concat.field_as_slice() {
                    res_concat.push(DummyDefault::default(), *from, *width);
                }
                m(res_concat.t_as_mut_slice(), this.operands());
                ConcatFields(res_concat)
            }
            StaticLut(v, table) => StaticLut(map1!(m, v), table.clone()),
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
            ArbMulAdd(v) => ArbMulAdd(map3!(m, v)),
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
                    DummyDefault::default(),
                    DummyDefault::default(),
                    DummyDefault::default(),
                    DummyDefault::default(),
                    DummyDefault::default(),
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
