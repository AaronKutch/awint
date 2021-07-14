use std::{num::NonZeroUsize, rc::Rc};

type P = std::rc::Rc<Op>;

/// Mimicking operation
#[derive(Debug, Hash, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Op {
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

use Op::*;

impl Op {
    /// List all sources used by `self`
    pub fn list_sources(&self) -> Vec<P> {
        let mut v = Vec::new();
        match self {
            OpaqueAssign(_) | ZeroAssign(_) | UmaxAssign(_) | ImaxAssign(_) | IminAssign(_)
            | UoneAssign(_) | LitAssign(_) => (),

            CopyAssign(p) => {
                v.push(Rc::clone(p));
            }
            ZeroResizeAssign(_, p) => {
                v.push(Rc::clone(p));
            }
            SignResizeAssign(_, p) => {
                v.push(Rc::clone(p));
            }
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
            Lz(p) => v.push(Rc::clone(p)),
            Tz(p) => v.push(Rc::clone(p)),
            CountOnes(p) => v.push(Rc::clone(p)),

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
            ShlAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            LshrAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            AshrAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            RotlAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            RotrAssign(p0, p1) => {
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

            Lut(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            Funnel(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            UQuoAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            URemAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            IQuoAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            IRemAssign(p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            ResizeAssign(_, p0, p1) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
            }
            Field(p0, p1, p2, p3, p4) => {
                v.push(Rc::clone(p0));
                v.push(Rc::clone(p1));
                v.push(Rc::clone(p2));
                v.push(Rc::clone(p3));
                v.push(Rc::clone(p4));
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
                | LitAssign(_)
        )
    }
}
