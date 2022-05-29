use awint_core::Bits;

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

pub fn chars_to_string(chars: &[char]) -> String {
    let mut s = String::new();
    for c in chars {
        s.push(*c);
    }
    s
}
