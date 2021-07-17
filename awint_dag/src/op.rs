/// Mimicking operation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
#[non_exhaustive]
pub enum Op {
    // literal assign
    Literal(awint_ext::ExtAwi),

    // no dependence on any `self`
    ZeroAssign,
    UmaxAssign,
    ImaxAssign,
    IminAssign,
    UoneAssign,

    // represents an unknown, arbitrary, or opaque-boxed source
    OpaqueAssign,

    // Assigns the bitwidth value
    //BwAssign,

    // these are special because although they take `&mut self`, the value of `self` is completely
    // overridden, so there is no dependency on `self.op()`.
    ResizeAssign,
    ZeroResizeAssign,
    ZeroResizeAssignOverflow,
    SignResizeAssign,
    SignResizeAssignOverflow,
    // I'm not sure what to do about dynamic `None` cases which would depend on `self`
    CopyAssign,
    Lut,
    Funnel,
    UQuoAssign,
    URemAssign,
    IQuoAssign,
    IRemAssign,
    MulAddTriop,
    CinSumTriop,

    // (&mut self)
    NotAssign,
    RevAssign,
    NegAssign,
    AbsAssign,

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
    OrAssign,
    AndAssign,
    XorAssign,
    ShlAssign,
    LshrAssign,
    AshrAssign,
    RotlAssign,
    RotrAssign,
    AddAssign,
    SubAssign,
    RsbAssign,

    // (&self, rhs: &Self) -> Option<bool>
    ConstEq,
    ConstNe,
    Ult,
    Ule,
    Ugt,
    Uge,
    Ilt,
    Ile,
    Igt,
    Ige,

    IncAssign,
    IncAssignCout,
    DecAssign,
    DecAssignCout,

    // also used in `cin_sum_triop`
    UnsignedOverflow,
    SignedOverflow,

    Field,
}
