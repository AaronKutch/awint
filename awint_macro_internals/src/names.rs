/// Prefixes used for codegen names and functions. Most of these should be
/// prefixed with two underscores and the crate name to prevent collisions.
#[derive(Debug, Clone, Copy)]
pub struct Names<'a> {
    /// Prefix used for constants
    pub constant: &'a str,
    /// Prefix used for initial bindings
    pub bind: &'a str,
    /// Prefix used for values
    pub value: &'a str,
    /// Prefix used for widths
    pub width: &'a str,
    /// Prefix used for concatenation width
    pub cw: &'a str,
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
const BIND: &str = "__awint_bind";
const VALUE: &str = "__awint_val";
const WIDTH: &str = "__awint_width";
const CW: &str = "__awint_cw";
const BITS_REF: &str = "__awint_ref";
const AWI: &str = "__awint_awi";
const AWI_REF: &str = "__awint_awi_ref";
const SHL: &str = "__awint_shl";

/// Default names for `awint`
pub const AWINT_NAMES: Names = Names {
    constant: CONSTANT,
    bind: BIND,
    value: VALUE,
    width: WIDTH,
    cw: CW,
    bits_ref: BITS_REF,
    awi: AWI,
    awi_ref: AWI_REF,
    shl: SHL,
};

#[derive(Debug, Clone, Copy)]
pub struct FnNames<'a> {
    pub lt_fn: &'a str,
    pub common_lt_fn: &'a str,
    pub common_ne_fn: &'a str,
    pub max_fn: &'a str,
    pub copy_assign: &'a str,
    pub field: &'a str,
    pub field_to: &'a str,
    pub field_from: &'a str,
    pub unwrap: &'a str,
}

// TODO instead should probably go the single `unstable_` function route and
// handle target debug assertion configuration for unwraps that way
const UNWRAP: &str = ".unwrap()";

pub const AWINT_FN_NAMES: FnNames = FnNames {
    lt_fn: "Bits::unstable_lt_checks",
    common_lt_fn: "Bits::unstable_common_lt_checks",
    common_ne_fn: "Bits::unstable_common_ne_checks",
    max_fn: "Bits::unstable_max",
    copy_assign: "Bits::copy_assign",
    field: "Bits::field",
    field_to: "Bits::field_to",
    field_from: "Bits::field_from",
    unwrap: UNWRAP,
};
