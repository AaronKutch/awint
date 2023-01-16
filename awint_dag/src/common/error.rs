#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvalError {
    // An operand points nowhere, so the DAG is broken
    InvalidPtr,
    // Thrown if a `Literal`, `Invalid`, or `Opaque` node is attempted to be evaluated
    Unevaluatable,
    WrongNumberOfOperands,
    // An `Opaque` node was expected
    ExpectedOpaque,
    // an operand is not a `Literal`
    NonliteralOperand,
    // wrong bitwidths of operands
    WrongBitwidth,
    // Something needs a statically known bitwidth
    NonStaticBitwidth,
    // wrong integer value of an operand, such as overshifting from a shift operation or going out
    // of bounds in a field operation
    InvalidOperandValue,
    // A typical `Bits` operation failed
    EvalFailure,
    // An operation was unimplemented
    Unimplemented,
    // Some other kind of brokenness, such as dependency edges not agreeing with operand edges
    OtherStr(&'static str),
    OtherString(String),
    AssertionFailure(String),
}
