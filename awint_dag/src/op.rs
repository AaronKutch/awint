/// Mimicking operation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum Op {
    // literal assign
    Literal(awint_ext::ExtAwi),

    // represents an unknown, arbitrary, or opaque-boxed source
    Opaque,

    // the bitwidth value
    //Bw,

    // these are special because although they take `&mut self`, the value of `self` is completely
    // overridden, so there is no dependency on `self.op()`.
    Resize,
    ZeroResize,
    ZeroResizeOverflow,
    SignResize,
    SignResizeOverflow,
    // I'm not sure what to do about dynamic `None` cases which would depend on `self`
    Copy,
    Lut,
    Funnel,
    UQuo,
    URem,
    IQuo,
    IRem,
    MulAdd,
    CinSum,
    UnsignedOverflow,
    SignedOverflow,

    // (&mut self)
    Not,
    Rev,
    Neg,
    Abs,

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
    Or,
    And,
    Xor,
    Shl,
    Lshr,
    Ashr,
    Rotl,
    Rotr,
    Add,
    Sub,
    Rsb,

    // (&self, rhs: &Self) -> Option<bool>
    Eq,
    Ne,
    Ult,
    Ule,
    Ugt,
    Uge,
    Ilt,
    Ile,
    Igt,
    Ige,

    Inc,
    IncCout,
    Dec,
    DecCout,

    LutSet,
    Field,
}

use Op::*;

impl Op {
    /// Returns the name of the operation
    pub fn operation_name(&self) -> &'static str {
        match *self {
            Literal(_) => "literal",
            Opaque => "opaque",
            Resize => "resize",
            ZeroResize => "zero_resize",
            ZeroResizeOverflow => "zero_reisze_overflow",
            SignResize => "sign_resize",
            SignResizeOverflow => "sign_resize_overflow",
            Copy => "copy",
            Lut => "lut",
            Funnel => "funnel",
            UQuo => "uquo",
            URem => "urem",
            IQuo => "iquo",
            IRem => "irem",
            MulAdd => "mul_add",
            CinSum => "cin_sum",
            UnsignedOverflow => "unsigned_overflow",
            SignedOverflow => "signed_overflow",
            Not => "not",
            Rev => "rev",
            Neg => "neg",
            Abs => "abs",
            IsZero => "is_zero",
            IsUmax => "is_umax",
            IsImax => "is_imax",
            IsImin => "is_imin",
            IsUone => "is_uone",
            Lsb => "lsb",
            Msb => "msb",
            Lz => "lz",
            Tz => "tz",
            CountOnes => "count_ones",
            Or => "or",
            And => "and",
            Xor => "xor",
            Shl => "shl",
            Lshr => "lshr",
            Ashr => "ashr",
            Rotl => "rotl",
            Rotr => "rotr",
            Add => "add",
            Sub => "sub",
            Rsb => "rsb",
            Eq => "eq",
            Ne => "ne",
            Ult => "ult",
            Ule => "ule",
            Ugt => "ugt",
            Uge => "uge",
            Ilt => "ilt",
            Ile => "ile",
            Igt => "igt",
            Ige => "ige",
            Inc => "inc",
            IncCout => "inc_cout",
            Dec => "dec",
            DecCout => "dec_cout",
            LutSet => "lut_set",
            Field => "field",
        }
    }

    /// Returns names of operands
    pub fn operand_names(&self) -> Vec<&'static str> {
        let mut v = vec![];
        // add common "lhs"
        match *self {
            Literal(_) | Opaque => (),

            Resize => {
                v.push("x");
                v.push("extension");
            }
            ZeroResize | SignResize | ZeroResizeOverflow | SignResizeOverflow => {
                v.push("x");
            }
            Copy => {
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
            UQuo | URem | IQuo | IRem => {
                v.push("duo");
                v.push("div");
            }
            MulAdd => {
                v.push("lhs");
                v.push("rhs");
            }
            CinSum | UnsignedOverflow | SignedOverflow => {
                v.push("cin");
                v.push("lhs");
                v.push("rhs");
            }

            Not | Rev | Neg | Abs => v.push("x"),

            IsZero | IsUmax | IsImax | IsImin | IsUone | Lsb | Msb => v.push("x"),

            Lz | Tz | CountOnes => v.push("x"),

            Or | And | Xor | Shl | Lshr | Ashr | Rotl | Rotr | Add | Sub | Rsb => {
                v.push("lhs");
                v.push("rhs")
            }

            Eq | Ne | Ult | Ule | Ugt | Uge | Ilt | Ile | Igt | Ige => {
                v.push("lhs");
                v.push("rhs");
            }

            Inc | IncCout | DecCout | Dec => {
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
