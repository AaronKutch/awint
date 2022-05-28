/// Prefixes used for codegen names and functions. Most of these should be
/// prefixed with two underscores and the crate name to prevent collisions.
pub struct Names<'a> {
    /// Prefix used for constants
    pub constant: &'a str,
    /// Prefix used for initial bindings
    pub binding: &'a str,
    /// Prefix used for values
    pub value: &'a str,
    /// Prefix used for widths
    pub width: &'a str,
    /// Prefix used for common bitwidth
    pub bw: &'a str,
    /// Prefix used for `Bits` references
    pub bits_ref: &'a str,
    /// Name used by the construct which might be created for returning, created
    /// as a temporary only, or never created.
    pub awi: &'a str,
    /// Name used for the reference to `awi`
    pub awi_ref: &'a str,
    /// Name used for the fielding `to` offset
    pub shl: &'a str,
}

const CONSTANT: &str = "__awint_constant";
const BINDING: &str = "__awint_bind";
const VALUE: &str = "__awint_val";
const WIDTH: &str = "__awint_width";
const BW: &str = "__awint_bw";
const BITS_REF: &str = "__awint_ref";
const AWI: &str = "__awint_awi";
const AWI_REF: &str = "__awint_awi_ref";
const SHL: &str = "__awint_shl";

/// Default names for `awint`
pub const AWINT_NAMES: Names = Names {
    constant: CONSTANT,
    binding: BINDING,
    value: VALUE,
    width: WIDTH,
    bw: BW,
    bits_ref: BITS_REF,
    awi: AWI,
    awi_ref: AWI_REF,
    shl: SHL,
};
