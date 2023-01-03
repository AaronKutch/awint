use std::num::NonZeroUsize;

use awint_ext::ExtAwi;

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
    /// Name used for CC checking result
    pub res: &'a str,
}

/// Default names for `awint`
pub const AWINT_NAMES: Names = Names {
    constant: "__awint_constant",
    bind: "__awint_bind",
    value: "__awint_val",
    width: "__awint_width",
    cw: "__awint_cw",
    bits_ref: "__awint_ref",
    awi: "__awint_awi",
    awi_ref: "__awint_awi_ref",
    shl: "__awint_shl",
    res: "__awint_res",
};

#[derive(Debug, Clone, Copy)]
pub struct FnNames<'a> {
    pub get_bw: &'a str,
    pub mut_bits_ref: &'a str,
    pub bits_ref: &'a str,
    pub usize_cast: &'a str,
    pub usize_add: &'a str,
    pub usize_sub: &'a str,
    pub max_fn: &'a str,
    pub cc_checks_fn: &'a str,
    pub copy_: &'a str,
    pub field: &'a str,
    pub field_to: &'a str,
    pub field_from: &'a str,
    pub field_width: &'a str,
    pub field_bit: &'a str,
    pub bw_call: &'a [char],
}

pub const AWINT_FN_NAMES: FnNames = FnNames {
    get_bw: "Bits::bw",
    mut_bits_ref: "&mut Bits",
    bits_ref: "&Bits",
    usize_cast: "Bits::usize_cast",
    usize_add: "Bits::usize_add",
    usize_sub: "Bits::usize_sub",
    max_fn: "Bits::unstable_max",
    cc_checks_fn: "Bits::unstable_cc_checks",
    copy_: "Bits::copy_",
    field: "Bits::field",
    field_to: "Bits::field_to",
    field_from: "Bits::field_from",
    field_width: "Bits::field_width",
    field_bit: "Bits::field_bit",
    bw_call: &['.', 'b', 'w', '(', ')'],
};

/// Note: the type must be unambiguous for the construction functions
///
/// - `static_width`: if the type needs a statically known width
/// - `return_type`: if the bits need to be returned
/// - `must_use`: wraps return values in a function for insuring `#[must_use]`
/// - `lit_construction_fn`: construction function for known literals
/// - `construction_fn`: is input the specified initialization, width if it is
///   statically known, and dynamic width if known. As a special case, the
///   initialization is empty for when initialization doesn't matter
pub struct CodeGen<
    'a,
    F0: FnMut(&str) -> String,
    // I run into weird lifetime issues trying to use &Bits
    F1: FnMut(ExtAwi) -> String,
    F2: FnMut(&str, Option<NonZeroUsize>, Option<&str>) -> String,
> {
    pub static_width: bool,
    pub return_type: Option<&'a str>,
    pub must_use: F0,
    pub lit_construction_fn: F1,
    pub construction_fn: F2,
    pub fn_names: FnNames<'a>,
}
