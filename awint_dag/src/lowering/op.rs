use std::{collections::HashMap, num::NonZeroUsize, rc::Rc};

use Op::*;

use crate::{
    lowering::{Ptr, PtrEqRc},
    mimick,
};

type P = crate::lowering::Ptr;
type MP = mimick::Op;

/// Intermediate Operation for lowering from the mimicking operation to lut-only
/// form
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Op {
    Unlowered(Rc<MP>),

    // represents an unknown, arbitrary, or opaque-boxed source
    OpaqueAssign(NonZeroUsize),

    // these are special because although they take `&mut self`, the value of `self` is completely
    // overridden, so there is no dependency on `self.op()`.
    ResizeAssign(NonZeroUsize, P, P),
    ZeroResizeAssign(NonZeroUsize, P),
    SignResizeAssign(NonZeroUsize, P),
    // I'm not sure what to do about dynamic `None` cases which would depend on `self`
    CopyAssign(P),
    Lut(P, P),
    Funnel(P, P),
    UQuoAssign(P, P),
    URemAssign(P, P),
    IQuoAssign(P, P),
    IRemAssign(P, P),

    // no dependence on any `self`
    ZeroAssign(NonZeroUsize),
    UmaxAssign(NonZeroUsize),
    ImaxAssign(NonZeroUsize),
    IminAssign(NonZeroUsize),
    UoneAssign(NonZeroUsize),

    // literal assign
    LitAssign(awint_ext::ExtAwi),

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

    // (&self) -> usize
    Lz(P),
    Tz(P),
    CountOnes(P),

    // (&mut self, rhs: &Self)
    OrAssign(P, P),
    AndAssign(P, P),
    XorAssign(P, P),
    ShlAssign(P, P),
    LshrAssign(P, P),
    AshrAssign(P, P),
    RotlAssign(P, P),
    RotrAssign(P, P),
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

    Field(P, P, P, P, P),
}

macro_rules! single_bw {
    ($self:ident, $p:ident, $($enum_var:ident)*) => {
        match $p.as_ref() {
            $(
                MP::$enum_var(w) => return Some($enum_var(*w)),
            )*
            _ => (),
        }
    };
}

macro_rules! unary {
    ($self:ident, $p:ident, $mp_to_p:ident, $($enum_var:ident)*) => {
        match $p.as_ref() {
            $(
                MP::$enum_var(p) => return Some($enum_var($mp_to_p[&PtrEqRc(Rc::clone(p))])),
            )*
            _ => (),
        }
    };
}

macro_rules! binary {
    ($self:ident, $p:ident, $mp_to_p:ident, $($enum_var:ident)*) => {
        match $p.as_ref() {
            $(
                MP::$enum_var(p0, p1) => return Some(
                    $enum_var($mp_to_p[&PtrEqRc(Rc::clone(p0))], $mp_to_p[&PtrEqRc(Rc::clone(p1))])
                ),
            )*
            _ => (),
        }
    };
}

impl Op {
    pub fn is_initialization(&self) -> bool {
        match self {
            Unlowered(p) => p.is_initialization(),
            OpaqueAssign(_) | ZeroAssign(_) | UmaxAssign(_) | ImaxAssign(_) | IminAssign(_)
            | UoneAssign(_) | LitAssign(_) => true,
            _ => false,
        }
    }

    pub fn lower(&self, mp_to_p: &HashMap<PtrEqRc, Ptr>) -> Option<Self> {
        if let Unlowered(p) = self {
            match p.as_ref() {
                MP::ResizeAssign(w, p0, p1) => {
                    return Some(ResizeAssign(
                        *w,
                        mp_to_p[&PtrEqRc(Rc::clone(p0))],
                        mp_to_p[&PtrEqRc(Rc::clone(p1))],
                    ))
                }
                MP::ZeroResizeAssign(w, p) => {
                    return Some(ZeroResizeAssign(*w, mp_to_p[&PtrEqRc(Rc::clone(p))]))
                }
                MP::SignResizeAssign(w, p) => {
                    return Some(SignResizeAssign(*w, mp_to_p[&PtrEqRc(Rc::clone(p))]))
                }
                MP::LitAssign(awi) => return Some(LitAssign(awi.clone())),
                MP::Field(p0, p1, p2, p3, p4) => {
                    return Some(Field(
                        mp_to_p[&PtrEqRc(Rc::clone(p0))],
                        mp_to_p[&PtrEqRc(Rc::clone(p1))],
                        mp_to_p[&PtrEqRc(Rc::clone(p2))],
                        mp_to_p[&PtrEqRc(Rc::clone(p3))],
                        mp_to_p[&PtrEqRc(Rc::clone(p4))],
                    ))
                }
                _ => (),
            }
            single_bw!(
                self, p, OpaqueAssign ZeroAssign UmaxAssign ImaxAssign IminAssign UoneAssign
            );
            unary!(
                self, p, mp_to_p, CopyAssign NotAssign RevAssign NegAssign AbsAssign IsZero IsUmax
                IsImax IsImin IsUone Lsb Msb Lz Tz CountOnes
            );
            binary!(
                self, p, mp_to_p, Lut Funnel UQuoAssign URemAssign IQuoAssign IRemAssign OrAssign
                AndAssign XorAssign ShlAssign LshrAssign AshrAssign RotlAssign RotrAssign AddAssign
                SubAssign RsbAssign ConstEq ConstNe Ult Ule Ugt Uge Ilt Ile Igt Ige
            );
        }
        None
    }
}
