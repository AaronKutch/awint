use core::fmt;
use std::fmt::Debug;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum EvalError {
    // An operand points nowhere, so the DAG is broken
    #[error("InvalidPtr")]
    InvalidPtr,
    // Thrown if a `Literal`, `Invalid`, or `Opaque` node is attempted to be evaluated
    #[error("Unevaluatable")]
    Unevaluatable,
    #[error("WrongNumberOfOperands")]
    WrongNumberOfOperands,
    // An `Opaque` node was expected
    #[error("ExpectedOpaque")]
    ExpectedOpaque,
    // an operand is not a `Literal`
    #[error("NonliteralOperand")]
    NonliteralOperand,
    // wrong bitwidths of operands
    #[error("WrongBitwidth")]
    WrongBitwidth,
    // Something needs a statically known bitwidth
    #[error("NonStaticBitwidth")]
    NonStaticBitwidth,
    // wrong integer value of an operand, such as overshifting from a shift operation or going out
    // of bounds in a field operation
    #[error("InvalidOperandValue")]
    InvalidOperandValue,
    // A typical `Bits` operation failed
    #[error("EvalFailure")]
    EvalFailure,
    // An operation was unimplemented
    #[error("Unimplemented")]
    Unimplemented,
    // Some other kind of brokenness, such as dependency edges not agreeing with operand edges
    #[error("{0}")]
    OtherStr(&'static str),
    #[error("{0}")]
    OtherString(String),
    #[error("AssertionFailure({0})")]
    AssertionFailure(String),
}

struct DisplayStr<'a>(pub &'a str);
impl<'a> Debug for DisplayStr<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}

impl Debug for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPtr => write!(f, "InvalidPtr"),
            Self::Unevaluatable => write!(f, "Unevaluatable"),
            Self::WrongNumberOfOperands => write!(f, "WrongNumberOfOperands"),
            Self::ExpectedOpaque => write!(f, "ExpectedOpaque"),
            Self::NonliteralOperand => write!(f, "NonliteralOperand"),
            Self::WrongBitwidth => write!(f, "WrongBitwidth"),
            Self::NonStaticBitwidth => write!(f, "NonStaticBitwidth"),
            Self::InvalidOperandValue => write!(f, "InvalidOperandValue"),
            Self::EvalFailure => write!(f, "EvalFailure"),
            Self::Unimplemented => write!(f, "Unimplemented"),
            Self::OtherStr(arg0) => f.debug_tuple("OtherStr").field(&DisplayStr(arg0)).finish(),
            Self::OtherString(arg0) => f
                .debug_tuple("OtherString")
                .field(&DisplayStr(arg0))
                .finish(),
            Self::AssertionFailure(arg0) => f
                .debug_tuple("AssertionFailure")
                .field(&DisplayStr(arg0))
                .finish(),
        }
    }
}
