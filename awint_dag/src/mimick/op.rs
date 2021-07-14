use std::{num::NonZeroUsize, rc::Rc};

use crate::{mimick::Lineage, primitive as prim};

type P = std::rc::Rc<Op>;

/// Mimicking operation
#[derive(Debug, Hash, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Op {
    // represents an unknown, arbitrary, or opaque-boxed source
    OpaqueAssign(NonZeroUsize),

    // no dependence on previous value of `self`
    ZeroAssign(NonZeroUsize),
    UmaxAssign(NonZeroUsize),
    ImaxAssign(NonZeroUsize),
    IminAssign(NonZeroUsize),
    UoneAssign(NonZeroUsize),

    // Literal assignments used by the `From<core::primitive::*> for awint_dag::primitive::*`
    // impls
    LitUsizeAssign(usize),
    LitIsizeAssign(isize),
    LitU8Assign(u8),
    LitI8Assign(i8),
    LitU16Assign(u16),
    LitI16Assign(i16),
    LitU32Assign(u32),
    LitI32Assign(i32),
    LitU64Assign(u64),
    LitI64Assign(i64),
    LitU128Assign(u128),
    LitI128Assign(i128),
    LitBoolAssign(bool),

    // used by `unstable_from_slice`
    LitRawSliceAssign(Vec<usize>),

    // regular assignments
    UsizeAssign(prim::usize),
    IsizeAssign(prim::isize),
    U8Assign(prim::u8),
    I8Assign(prim::i8),
    U16Assign(prim::u16),
    I16Assign(prim::i16),
    U32Assign(prim::u32),
    I32Assign(prim::i32),
    U64Assign(prim::u64),
    I64Assign(prim::i64),
    U128Assign(prim::u128),
    I128Assign(prim::i128),
    BoolAssign(prim::bool),

    // (&mut self)
    NotAssign(P),
    RevAssign(P),
    NegAssign(P),
    AbsAssign(P),

    // (&self) -> bool
    IsZero(P),
    IsUmax(P),
    IsImax(P),
    IsImin(P),
    IsUone(P),
    Lsb(P),
    Msb(P),

    // (&self) -> other
    ToUsize(P),
    ToIsize(P),
    ToU8(P),
    ToI8(P),
    ToU16(P),
    ToI16(P),
    ToU32(P),
    ToI32(P),
    ToU64(P),
    ToI64(P),
    ToU128(P),
    ToI128(P),
    ToBool(P),

    // (&self) -> usize
    Lz(P),
    Tz(P),
    CountOnes(P),

    // (&mut self, rhs: &Self)
    CopyAssign(P, P),
    OrAssign(P, P),
    AndAssign(P, P),
    XorAssign(P, P),
    //ShlAssign(P, P),
    //LshrAssign(P, P),
    //AshrAssign(P, P),
    //RotlAssign(P, P),
    //RotrAssign(P, P),
    AddAssign(P, P),
    SubAssign(P, P),
    RsbAssign(P, P),

    // (&self, rhs: &Self) -> Option<bool>
    ConstEq(P, P),
    ConstNe(P, P),
    Ult(P, P),
    Ule(P, P),
    Ugt(P, P),
    Uge(P, P),
    Ilt(P, P),
    Ile(P, P),
    Igt(P, P),
    Ige(P, P),

    Lut(P, P, P),
    Field(P, prim::usize, P, prim::usize, prim::usize),
    ResizeAssign(P, P, prim::bool),
    Funnel(P, P, P),
    UQuoAssign(P, P, P),
    URemAssign(P, P, P),
    IQuoAssign(P, P, P),
    IRemAssign(P, P, P),
}

use Op::*;

impl Op {
    /// List all sources used by `self`
    pub fn list_sources(&self) -> Vec<P> {
        let mut v = Vec::new();
        match self {
            OpaqueAssign(_) | ZeroAssign(_) | UmaxAssign(_) | ImaxAssign(_) | IminAssign(_)
            | UoneAssign(_) | LitUsizeAssign(_) | LitIsizeAssign(_) | LitU8Assign(_)
            | LitI8Assign(_) | LitU16Assign(_) | LitI16Assign(_) | LitU32Assign(_)
            | LitI32Assign(_) | LitU64Assign(_) | LitI64Assign(_) | LitU128Assign(_)
            | LitI128Assign(_) | LitBoolAssign(_) | LitRawSliceAssign(_) => (),

            UsizeAssign(p) => v.push(p.op()),
            IsizeAssign(p) => v.push(p.op()),
            U8Assign(p) => v.push(p.op()),
            I8Assign(p) => v.push(p.op()),
            U16Assign(p) => v.push(p.op()),
            I16Assign(p) => v.push(p.op()),
            U32Assign(p) => v.push(p.op()),
            I32Assign(p) => v.push(p.op()),
            U64Assign(p) => v.push(p.op()),
            I64Assign(p) => v.push(p.op()),
            U128Assign(p) => v.push(p.op()),
            I128Assign(p) => v.push(p.op()),
            BoolAssign(p) => v.push(p.op()),

            NotAssign(p) => v.push(Rc::clone(p)),
            RevAssign(p) => v.push(Rc::clone(p)),
            NegAssign(p) => v.push(Rc::clone(p)),
            AbsAssign(p) => v.push(Rc::clone(p)),
            IsZero(p) => v.push(Rc::clone(p)),
            IsUmax(p) => v.push(Rc::clone(p)),
            IsImax(p) => v.push(Rc::clone(p)),
            IsImin(p) => v.push(Rc::clone(p)),
            IsUone(p) => v.push(Rc::clone(p)),
            Lsb(p) => v.push(Rc::clone(p)),
            Msb(p) => v.push(Rc::clone(p)),
            ToUsize(p) => v.push(Rc::clone(p)),
            ToIsize(p) => v.push(Rc::clone(p)),
            ToU8(p) => v.push(Rc::clone(p)),
            ToI8(p) => v.push(Rc::clone(p)),
            ToU16(p) => v.push(Rc::clone(p)),
            ToI16(p) => v.push(Rc::clone(p)),
            ToU32(p) => v.push(Rc::clone(p)),
            ToI32(p) => v.push(Rc::clone(p)),
            ToU64(p) => v.push(Rc::clone(p)),
            ToI64(p) => v.push(Rc::clone(p)),
            ToU128(p) => v.push(Rc::clone(p)),
            ToI128(p) => v.push(Rc::clone(p)),
            ToBool(p) => v.push(Rc::clone(p)),
            Lz(p) => v.push(Rc::clone(p)),
            Tz(p) => v.push(Rc::clone(p)),
            CountOnes(p) => v.push(Rc::clone(p)),

            CopyAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            OrAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            AndAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            XorAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            AddAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            SubAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            RsbAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            ConstEq(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            ConstNe(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            Ult(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            Ule(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            Ugt(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            Uge(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            Ilt(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            Ile(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            Igt(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            Ige(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }

            Lut(p0, p1, p2) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
                v.push(Rc::clone(p2));
            }
            Funnel(p0, p1, p2) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
                v.push(Rc::clone(p2));
            }
            UQuoAssign(p0, p1, p2) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
                v.push(Rc::clone(p2));
            }
            URemAssign(p0, p1, p2) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
                v.push(Rc::clone(p2));
            }
            IQuoAssign(p0, p1, p2) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
                v.push(Rc::clone(p2));
            }
            IRemAssign(p0, p1, p2) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
                v.push(Rc::clone(p2));
            }
            ResizeAssign(p0, p1, p2) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
                v.push(p2.op());
            }
            Field(p0, p1, p2, p3, p4) => {
                v.push(Rc::clone(p0));
                v.push(p1.op());
                v.push(Rc::clone(p2));
                v.push(p3.op());
                v.push(p4.op());
            }
        }
        v
    }

    /// Returns if this `Op` is an initialization with no dependence on other
    /// sources. `OpaqueWithBw` returns `true`.
    pub fn is_initialization(&self) -> bool {
        matches!(
            self,
            OpaqueAssign(_)
                | ZeroAssign(_)
                | UmaxAssign(_)
                | ImaxAssign(_)
                | IminAssign(_)
                | UoneAssign(_)
                | LitUsizeAssign(_)
                | LitIsizeAssign(_)
                | LitU8Assign(_)
                | LitI8Assign(_)
                | LitU16Assign(_)
                | LitI16Assign(_)
                | LitU32Assign(_)
                | LitI32Assign(_)
                | LitU64Assign(_)
                | LitI64Assign(_)
                | LitU128Assign(_)
                | LitI128Assign(_)
                | LitBoolAssign(_)
                | LitRawSliceAssign(_)
        )
    }
}
