use crate::primitive as prim;

type P = std::rc::Rc<Op>;

/// Mimicking operation
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Op {
    // (&mut self) but with no dependence on previous value of `self`
    ZeroAssign,
    UmaxAssign,
    ImaxAssign,
    IminAssign,
    UoneAssign,

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
