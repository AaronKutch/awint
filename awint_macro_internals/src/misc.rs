use std::num::NonZeroUsize;

use awint_ext::{Bits, ExtAwi};
use triple_arena::ptr_struct;

#[cfg(feature = "debug")]
ptr_struct!(PText; PBind; PVal; PWidth; PCWidth);

// we should never need 4 billion entries for these macros
#[cfg(not(feature = "debug"))]
ptr_struct!(PText[u32](); PBind[u32](); PVal[u32](); PWidth[u32](); PCWidth[u32]());

pub fn i128_to_usize(x: i128) -> Result<usize, String> {
    usize::try_from(x).map_err(|_| "`usize::try_from` overflow".to_owned())
}

pub fn i128_to_nonzerousize(x: i128) -> Result<NonZeroUsize, String> {
    NonZeroUsize::new(i128_to_usize(x)?).ok_or_else(|| "`NonZeroUsize::new` overflow".to_owned())
}

pub fn usize_to_i128(x: usize) -> Result<i128, String> {
    i128::try_from(x).map_err(|_| "`i128::try_from` overflow".to_owned())
}

pub fn chars_to_string(chars: &[char]) -> String {
    let mut s = String::new();
    for c in chars {
        s.push(*c);
    }
    s
}

/// Returns architecture-independent Rust code that returns an
/// `InlAwi` preset with the value of `bits`.
pub fn unstable_native_inlawi(bits: &Bits) -> String {
    // gets `bits` in `Vec<u8>` form, truncated
    let sig = bits.sig();
    let len = (sig / 8) + (((sig % 8) != 0) as usize);
    let mut buf = vec![0u8; len];
    bits.to_u8_slice(&mut buf);

    // this absolutely has to be done, because the proc-macro crate may be run on an
    // architecture with a different pointer width than the true target architecture
    // (and there is currently no way to get information about the true target
    // architecture from the build architecture, in fact the same output could
    // potentially be used on multiple architectures). `unstable_raw_digits` adjusts
    // `LEN` based on the native `usize` width, and `unstable_from_u8_slice` also
    // adjusts for big endian archiectures.
    format!(
        "InlAwi::<{},{{Bits::unstable_raw_digits({})}}>::unstable_from_u8_slice(&{:?})",
        bits.bw(),
        bits.bw(),
        buf,
    )
}

/// Returns architecture-independent Rust code that returns an
/// `InlAwi` type with bitwidth `w`.
pub fn unstable_native_inlawi_ty(w: u128) -> String {
    format!("InlAwi::<{w},{{Bits::unstable_raw_digits({w})}}>")
}

pub fn awint_must_use(s: &str) -> String {
    format!("Bits::must_use({s})")
}

pub fn awint_lit_construction_fn(awi: ExtAwi) -> String {
    unstable_native_inlawi(&awi)
}

pub fn awint_extawi_lit_construction_fn(awi: ExtAwi) -> String {
    format!("ExtAwi::from_bits(&{})", unstable_native_inlawi(&awi))
}

pub fn extawi_s(init: &str, s: &str) -> String {
    format!("ExtAwi::panicking_{init}({s})")
}

pub fn inlawi_s(init: &str, w: NonZeroUsize) -> String {
    format!("InlAwi::<{w},{{Bits::unstable_raw_digits({w})}}>::{init}()",)
}

pub fn cc_construction_fn(
    mut init: &str,
    static_width: Option<NonZeroUsize>,
    dynamic_width: Option<&str>,
) -> String {
    if init.is_empty() {
        init = "zero";
    }
    if let Some(w) = static_width {
        inlawi_s(init, w)
    } else if let Some(s) = dynamic_width {
        extawi_s(init, s)
    } else {
        unreachable!()
    }
}

pub fn inlawi_construction_fn(
    mut init: &str,
    static_width: Option<NonZeroUsize>,
    _dynamic_width: Option<&str>,
) -> String {
    if init.is_empty() {
        init = "zero";
    }
    if let Some(w) = static_width {
        inlawi_s(init, w)
    } else {
        unreachable!()
    }
}

pub fn extawi_construction_fn(
    mut init: &str,
    _static_width: Option<NonZeroUsize>,
    dynamic_width: Option<&str>,
) -> String {
    if init.is_empty() {
        init = "zero";
    }
    if let Some(s) = dynamic_width {
        extawi_s(init, s)
    } else {
        unreachable!()
    }
}
