#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
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
