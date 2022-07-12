#[derive(Debug, Clone)]
pub enum EvalError {
    // Thrown if a `Literal`, `Invalid`, or `Opaque` node is attempted to be evaluated
    Unevaluatable,
    // An `Opaque` node was expected
    ExpectedOpaque,
    // An operand points nowhere, so the DAG is broken
    InvalidPtr,
    // an operand is not a `Literal`
    NonliteralOperand,
    // wrong bitwidths of operands
    WrongBitwidth,
    // wrong integer value of an operand, such as overshifting from a shift operation or going out
    // of bounds in a field operation
    InvalidOperandValue,
    // Something needs a statically known bitwidth
    NonStaticBitwidth,
    // A function on a operation was unimplemented
    Unimplemented,
    // Some other kind of brokenness, such as dependency edges not agreeing with operand edges
    OtherStr(&'static str),
    OtherString(String),
}
