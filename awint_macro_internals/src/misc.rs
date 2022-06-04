use std::num::NonZeroUsize;

use awint_core::Bits;
use awint_ext::ExtAwi;

#[cfg(debug_assertions)]
triple_arena::ptr_trait_struct_with_gen!(PText; PBind; PVal; PWidth; PCWidth);

#[cfg(not(debug_assertions))]
triple_arena::ptr_trait_struct!(PText; PBind; PVal; PWidth; PCWidth);

/// Returns architecture-independent Rust code that returns an
/// `InlAwi` preset with the value of `bits`.
pub fn unstable_native_inlawi(bits: &Bits) -> String {
    // gets `bits` in `Vec<u8>` form, truncated
    let sig_bits = bits.bw() - bits.lz();
    let len = (sig_bits / 8) + (((sig_bits % 8) != 0) as usize);
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
/// `InlAwi` type with bitwidth `bw`.
pub fn unstable_native_inlawi_ty(bw: u128) -> String {
    format!("InlAwi::<{},{{Bits::unstable_raw_digits({})}}>", bw, bw,)
}

pub fn awint_must_use(s: &str) -> String {
    format!("Bits::must_use({})", s)
}

pub fn awint_lit_construction_fn(awi: ExtAwi) -> String {
    unstable_native_inlawi(&awi)
}

fn extawi_s(init: &str, s: &str) -> String {
    format!("ExtAwi::{}({})", init, s)
}

fn inlawi_s(init: &str, w: NonZeroUsize) -> String {
    format!(
        "InlAwi::<{},{{Bits::unstable_raw_digits({})}}>::{}()",
        w, w, init
    )
}

pub fn cc_construction_fn(
    _init: &str,
    static_width: Option<NonZeroUsize>,
    dynamic_width: Option<&str>,
) -> String {
    let init = "zero";
    if let Some(w) = static_width {
        inlawi_s(init, w)
    } else if let Some(s) = dynamic_width {
        extawi_s(init, s)
    } else {
        unreachable!()
    }
}

pub fn inlawi_construction_fn(
    _init: &str,
    static_width: Option<NonZeroUsize>,
    _dynamic_width: Option<&str>,
) -> String {
    let init = "zero";
    if let Some(w) = static_width {
        inlawi_s(init, w)
    } else {
        unreachable!()
    }
}

pub fn extawi_construction_fn(
    _init: &str,
    _static_width: Option<NonZeroUsize>,
    dynamic_width: Option<&str>,
) -> String {
    let init = "panicking_zero";
    if let Some(s) = dynamic_width {
        extawi_s(init, s)
    } else {
        unreachable!()
    }
}

pub fn cc_construction_fn2(
    init: &str,
    static_width: Option<NonZeroUsize>,
    dynamic_width: Option<&str>,
) -> String {
    if let Some(w) = static_width {
        inlawi_s(init, w)
    } else if let Some(s) = dynamic_width {
        extawi_s(init, s)
    } else {
        unreachable!()
    }
}

pub fn inlawi_construction_fn2(
    init: &str,
    static_width: Option<NonZeroUsize>,
    _dynamic_width: Option<&str>,
) -> String {
    if let Some(w) = static_width {
        inlawi_s(init, w)
    } else {
        unreachable!()
    }
}

pub fn extawi_construction_fn2(
    init: &str,
    _static_width: Option<NonZeroUsize>,
    dynamic_width: Option<&str>,
) -> String {
    let init = format!("panicking_{}", init);
    if let Some(s) = dynamic_width {
        extawi_s(&init, s)
    } else {
        unreachable!()
    }
}

pub fn chars_to_string(chars: &[char]) -> String {
    let mut s = String::new();
    for c in chars {
        s.push(*c);
    }
    s
}
