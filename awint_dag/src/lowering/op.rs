type P = crate::lowering::Ptr;

/// Intermediate Operation for lowering from the mimicking operation to lut-only
/// form
#[derive(Debug, Clone)]
#[non_exhaustive]
pub(crate) enum Op {
    // (&mut self) but with no dependence on previous value of `self`
    ZeroAssign,
    UmaxAssign,
    ImaxAssign,
    IminAssign,
    UoneAssign,

    // used by `unstable_from_slice`
    LitRawSliceAssign(Vec<usize>),

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
    Field(P, P, P, P, P),
    ResizeAssign(P, P, P),
    Funnel(P, P, P),
    UQuoAssign(P, P, P),
    URemAssign(P, P, P),
    IQuoAssign(P, P, P),
    IRemAssign(P, P, P),
}
