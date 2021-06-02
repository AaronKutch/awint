use std::rc::Rc;

use triple_arena::TriPtr as P;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Op {
    // (&mut self) but with no dependence on previous value of `self`
    ZeroAssign,
    UmaxAssign,
    ImaxAssign,
    IminAssign,
    UoneAssign,

    // special assignments
    BoolAssign(bool),
    UsizeAssign(usize),

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

    Lut(P, P, P),
    ResizeAssign(P, P, DagBool),
    Funnel(P, P, P),
    UQuoAssign(P, P, P),
    URemAssign(P, P, P),
    IQuoAssign(P, P, P),
    IRemAssign(P, P, P),

    // Used by `crate::primitive::*` for initializing copies.
    InitCopy(P),
}

macro_rules! dagprim {
    ($($name:ident $prim:ident),*,) => {
        $(
            #[derive(Debug, Clone)]
            pub enum $name {
                Core(core::primitive::$prim),
                Dag(Rc<crate::primitive::$prim>),
            }

            impl From<core::primitive::$prim> for $name {
                fn from(x: core::primitive::$prim) -> Self {
                    $name::Core(x)
                }
            }

            impl From<crate::primitive::$prim> for $name {
                fn from(x: crate::primitive::$prim) -> Self {
                    $name::Dag(Rc::new(x))
                }
            }
        )*
    };
}

dagprim!(
    DagBool bool,
    DagUsize usize,
    DagIsize isize,
    DagU8 u8,
    DagI8 i8,
    DagU16 u16,
    DagI16 i16,
    DagU32 u32,
    DagI32 i32,
    DagU64 u64,
    DagI64 i64,
    DagU128 u128,
    DagI128 i128,
);
