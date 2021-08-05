/// Mimicking operation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
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
    UnsignedOverflow,
    SignedOverflow,

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

    LutSet,
    Field,
}

use Op::*;

impl Op {
    /// Returns a tuple of mutable operand names and immutable operand names
    pub fn operand_stats(&self) -> Vec<&'static str> {
        let mut v = vec![];
        // add common "lhs"
        match *self {
            Literal(_) | ZeroAssign | UmaxAssign | ImaxAssign | IminAssign | UoneAssign
            | OpaqueAssign => (),

            ResizeAssign => {
                v.push("x");
                v.push("extension");
            }
            ZeroResizeAssign
            | SignResizeAssign
            | ZeroResizeAssignOverflow
            | SignResizeAssignOverflow => {
                v.push("x");
            }
            CopyAssign => {
                v.push("x");
            }
            Lut => {
                v.push("lut");
                v.push("inx")
            }
            Funnel => {
                v.push("x");
                v.push("s");
            }
            UQuoAssign | URemAssign | IQuoAssign | IRemAssign => {
                v.push("duo");
                v.push("div");
            }
            MulAddTriop => {
                v.push("lhs");
                v.push("rhs");
            }
            CinSumTriop | UnsignedOverflow | SignedOverflow => {
                v.push("cin");
                v.push("lhs");
                v.push("rhs");
            }

            NotAssign | RevAssign | NegAssign | AbsAssign => v.push("x"),

            IsZero | IsUmax | IsImax | IsImin | IsUone | Lsb | Msb => v.push("x"),

            Lz | Tz | CountOnes => v.push("x"),

            OrAssign | AndAssign | XorAssign | ShlAssign | LshrAssign | AshrAssign | RotlAssign
            | RotrAssign | AddAssign | SubAssign | RsbAssign => {
                v.push("lhs");
                v.push("rhs")
            }

            ConstEq | ConstNe | Ult | Ule | Ugt | Uge | Ilt | Ile | Igt | Ige => {
                v.push("lhs");
                v.push("rhs");
            }

            IncAssign | IncAssignCout | DecAssignCout | DecAssign => {
                v.push("x");
                v.push("cin");
            }
            LutSet => {
                v.push("lut");
                v.push("entry");
                v.push("inx");
            }
            Field => {
                v.push("lhs");
                v.push("to");
                v.push("rhs");
                v.push("from");
                v.push("width");
            }
        }
        v
    }
}
