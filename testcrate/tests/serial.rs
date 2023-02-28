use awint::{extawi, inlawi, Bits, ExtAwi, InlAwi, SerdeError::*, FP};

#[test]
fn string_max_fp() {
    // tests the 4096 cap
    assert_eq!(
        &format!("{:?}", FP::new(false, inlawi!(0), 4097).unwrap()),
        ""
    );
    assert_eq!(
        &format!("{:?}", FP::new(false, inlawi!(0), -4097).unwrap()),
        ""
    );
}

// non-const serialization tests
#[test]
fn string_conversion() {
    let x = "0i1".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 1);
    assert!(!x.to_bool());
    let x = "0u1".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 1);
    assert!(!x.to_bool());
    let x = "123i64".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 64);
    assert_eq!(x.to_i16(), 123);
    let x = "-123i64".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 64);
    assert_eq!(x.to_i16(), -123);
    let x = "123u64".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 64);
    assert_eq!(x.to_u16(), 123);
    let x = "1u1".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 1);
    assert_eq!(x.to_u16(), 1);
    let x = "-0xf1i16".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 16);
    assert_eq!(x.to_i16(), -0xf1);
    let x = "-0o71i16".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 16);
    assert_eq!(x.to_i16(), -0o71);
    let x = "-07i8".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 8);
    assert_eq!(x.to_i8(), -7);
    let x = "-00008i8".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 8);
    assert_eq!(x.to_i8(), -8);
    let x = "00008u8".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 8);
    assert_eq!(x.to_u8(), 8);
    let x = "010100111001".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 12);
    assert_eq!(x.to_u16(), 1337);
    let x = "0b_0101_0011_1001_u12".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 12);
    assert_eq!(x.to_u16(), 1337);
    let x = "1e10u64".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 64);
    assert_eq!(x.to_u64(), 10000000000);
    let x = "-1e10i64".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 64);
    assert_eq!(x.to_i64(), -10000000000);
    let x = "0b1e1111111u128".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 128);
    assert_eq!(x.to_u128(), 1u128 << 127);
    let x = "0b1e1111110i128".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 128);
    assert_eq!(x.to_i128(), 1i128 << 126);
    let x = "0x1_pf_u128".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 128);
    assert_eq!(x.to_u128(), 0x1000_0000_0000_0000);
    let x = "1u32f16".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 32);
    assert_eq!(x.to_u32(), 0x10000);
    let x = "1e-1u32f16".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 32);
    assert_eq!(x.to_u32(), 6554);
    let x = "65536u32f-16".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 32);
    assert_eq!(x.to_u32(), 1);
    let x = "0x0.00000001u32f48".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 32);
    assert_eq!(x.to_u32(), 0x10000);
    let x = "6.283185307e0u32f16".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 32);
    assert_eq!(x.to_u32(), 0x6487f);
    let x = "6_._283_185_307_e_0_u_32_f_16_".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 32);
    assert_eq!(x.to_u32(), 0x6487f);
    assert_eq!(
        FP::new(false, x, 16).unwrap().to_string(),
        "6.28319_u32f16".to_owned()
    );
    let x = "6.283185307_e4u32f16".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 32);
    assert_eq!(x.to_u32(), 0xf56fda63_u32);
    assert_eq!(
        FP::new(false, x, 16).unwrap().to_string(),
        "62831.85307_u32f16".to_owned()
    );
    let x = "6.283185307_e-2u32f16".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 32);
    assert_eq!(x.to_u32(), 0x1016_u32);
    assert_eq!(
        FP::new(false, x, 16).unwrap().to_string(),
        "0.06284_u32f16".to_owned()
    );
    let x = "-6.283185307_e-1i32f16".parse::<ExtAwi>().unwrap();
    assert_eq!(
        FP::new(true, x, 16).unwrap().to_string(),
        "-0.62831_i32f16".to_owned()
    );
    assert!(matches!("".parse::<ExtAwi>(), Err(Empty)));
    assert!(matches!("_".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("u".parse::<ExtAwi>(), Err(EmptyInteger)));
    assert!(matches!("123i".parse::<ExtAwi>(), Err(EmptyBitwidth)));
    assert!(matches!("123".parse::<ExtAwi>(), Err(EmptyBitwidth)));
    assert!(matches!("0u0".parse::<ExtAwi>(), Err(ZeroBitwidth)));
    assert!(matches!("0u-1".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("i64".parse::<ExtAwi>(), Err(EmptyInteger)));
    // we want to disallow this, since it could be interpreted as a subtraction of
    // some item starting with an underscore "- _...""
    assert!(matches!("-_0u1".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("-123i".parse::<ExtAwi>(), Err(EmptyBitwidth)));
    assert!(matches!("-123".parse::<ExtAwi>(), Err(EmptyBitwidth)));
    assert!(matches!("-i64".parse::<ExtAwi>(), Err(EmptyInteger)));
    assert!(matches!("-123u".parse::<ExtAwi>(), Err(NegativeUnsigned)));
    assert!(matches!("-u64".parse::<ExtAwi>(), Err(EmptyInteger)));
    assert!(matches!("-123u8".parse::<ExtAwi>(), Err(NegativeUnsigned)));
    assert!(matches!("-2i1".parse::<ExtAwi>(), Err(Overflow)));
    assert!(matches!("2u1".parse::<ExtAwi>(), Err(Overflow)));
    assert!(matches!("1i1".parse::<ExtAwi>(), Err(Overflow)));
    assert!(matches!("0xgu8".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("0xu8".parse::<ExtAwi>(), Err(EmptyInteger)));
    assert!(matches!("0x:u8".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("-0x_i8".parse::<ExtAwi>(), Err(EmptyInteger)));
    assert!(matches!("0.u8".parse::<ExtAwi>(), Err(Fractional)));
    assert!(matches!("0._u8f0".parse::<ExtAwi>(), Err(EmptyFraction)));
    assert!(matches!("0.u8f0".parse::<ExtAwi>(), Err(EmptyFraction)));
    assert!(matches!("0e-1u8".parse::<ExtAwi>(), Err(Fractional)));
    assert!(matches!("0e_u8f0".parse::<ExtAwi>(), Err(EmptyExponent)));
    assert!(matches!("0eu8f0".parse::<ExtAwi>(), Err(EmptyExponent)));
    assert!(matches!("0x0p_u8".parse::<ExtAwi>(), Err(EmptyExponent)));
    assert!(matches!("0x0pu8".parse::<ExtAwi>(), Err(EmptyExponent)));
    assert!(matches!("0x0p0u_".parse::<ExtAwi>(), Err(EmptyBitwidth)));
    assert!(matches!("0x0p0u".parse::<ExtAwi>(), Err(EmptyBitwidth)));
    assert!(matches!(
        "0x0p0i1f_".parse::<ExtAwi>(),
        Err(EmptyFixedPoint)
    ));
    assert!(matches!("0x0p0i1f".parse::<ExtAwi>(), Err(EmptyFixedPoint)));
    assert!(matches!("0b0e2u8f0".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("0o0e8u8f0".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("0x0pgu8f0".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("0epu8f0".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!(
        "0b1e10000000u128".parse::<ExtAwi>(),
        Err(Overflow)
    ));
    assert!(matches!("0b1e1111111i128".parse::<ExtAwi>(), Err(Overflow)));
}

macro_rules! fmt_test_inner {
    ($($awi:ident, $debug:expr, $display:expr, $bin:expr, $oct:expr, $lohex:expr, $hihex:expr);*;)
        => {
        $(
            assert_eq!(format!("{:?}", $awi), $debug);
            assert_eq!(format!("{}", $awi), $display);
            assert_eq!(format!("{:b}", $awi), $bin);
            assert_eq!(format!("{:o}", $awi), $oct);
            assert_eq!(format!("{:x}", $awi), $lohex);
            assert_eq!(format!("{:X}", $awi), $hihex);
        )*
    };
}

macro_rules! fmt_test {
    ($($awi:ident)*) => {
        $(
            fmt_test_inner!(
                $awi, "0xfedcba98_76543210_u100", "0xfedcba98_76543210_u100",
                "0b11111110_11011100_10111010_10011000_01110110_01010100_00110010_00010000_u100",
                "0o177334_56514166_25031020_u100", "0xfedcba98_76543210_u100",
                "0xFEDCBA98_76543210_u100";
            );
        )*
    }
}

#[test]
fn fmt_strings() {
    let inl_awi = inlawi!(0xfedcba9876543210u100);
    let ext_awi = extawi!(0xfedcba9876543210u100);
    let bits_awi = inlawi!(0xfedcba9876543210u100);
    let bits = bits_awi.as_ref();
    fmt_test!(inl_awi ext_awi bits);
    assert_eq!(format!("{}", inlawi!(0u100)), "0x0_u100");
    assert_eq!(
        format!("{}", inlawi!(0x1_fedcba98_76543210u100)),
        "0x1_fedcba98_76543210_u100"
    );

    let fpbits = FP::new(true, inlawi!(-0xabcd1234i36), 16).unwrap();
    assert_eq!(format!("{fpbits:x}"), "-0xabcd.1234_i36f16");
    assert_eq!(format!("{fpbits:X}"), "-0xABCD.1234_i36f16");
    assert_eq!(format!("{fpbits:o}"), "-0o125715.04432_i36f16");
    assert_eq!(
        format!("{fpbits:b}"),
        "-0b1010101111001101.00010010001101_i36f16"
    );
    assert_eq!(format!("{fpbits:?}"), "-43981.07111_i36f16");
    assert_eq!(format!("{fpbits}"), "-43981.07111_i36f16");

    let fpbits = FP::new(false, inlawi!(1u11), -16).unwrap();
    assert_eq!(format!("{fpbits}"), "65536.0_u11f-16");
    let fpbits = FP::new(false, inlawi!(1u11), 16).unwrap();
    assert_eq!(format!("{fpbits}"), "0.00002_u11f16");
    let fpbits = FP::new(false, inlawi!(11111111111), 16).unwrap();
    assert_eq!(format!("{fpbits}"), "0.03123_u11f16");
}

#[cfg(not(miri))]
#[test]
fn all_hex_byte_combos() {
    // keep at least 4 digits for in the future if we use SWAR
    let mut s = [b'0'; 67];
    let mut awi = inlawi!(0u268);
    let mut pad0 = inlawi!(0u268);
    let mut pad1 = inlawi!(0u268);
    let mut tmp = inlawi!(0u268);
    for i in 0..s.len() {
        for b in 0..=u8::MAX {
            s[s.len() - 1 - i] = b;
            match b {
                b'0'..=b'9' => {
                    awi.bytes_radix_(None, &s, 16, &mut pad0, &mut pad1)
                        .unwrap();
                    tmp.u8_(b - b'0');
                    tmp.shl_(i * 4).unwrap();
                    assert!(awi == tmp);
                }
                b'a'..=b'f' => {
                    awi.bytes_radix_(None, &s, 16, &mut pad0, &mut pad1)
                        .unwrap();
                    tmp.u8_(b - b'a' + 10);
                    tmp.shl_(i * 4).unwrap();
                    assert!(awi == tmp);
                }
                b'A'..=b'F' => {
                    awi.bytes_radix_(None, &s, 16, &mut pad0, &mut pad1)
                        .unwrap();
                    tmp.u8_(b - b'A' + 10);
                    tmp.shl_(i * 4).unwrap();
                    assert!(awi == tmp);
                }
                b'_' => {
                    awi.bytes_radix_(None, &s, 16, &mut pad0, &mut pad1)
                        .unwrap();
                    assert!(awi.is_zero());
                }
                _ => {
                    awi.bytes_radix_(None, &s, 16, &mut pad0, &mut pad1)
                        .unwrap_err();
                }
            }
            // set back
            s[s.len() - 1 - i] = b'0';
        }
    }
}

#[cfg(not(miri))]
#[test]
fn all_single_byte_combos() {
    let mut s = [b'0'; 1];
    let mut awi = inlawi!(0u8);
    let mut pad0 = inlawi!(0u8);
    let mut pad1 = inlawi!(0u8);
    let mut tmp = inlawi!(0u8);
    for r in 2..=36 {
        for b in 0..=u8::MAX {
            s[0] = b;
            let res = awi.bytes_radix_(None, &s, r, &mut pad0, &mut pad1);
            match b {
                b'0'..=b'9' => {
                    let v = b - b'0';
                    if v < r {
                        res.unwrap();
                        tmp.u8_(v);
                        assert!(awi == tmp);
                    } else {
                        res.unwrap_err();
                    }
                }
                b'a'..=b'z' => {
                    let v = b - b'a' + 10;
                    if v < r {
                        res.unwrap();
                        tmp.u8_(v);
                        assert!(awi == tmp);
                    } else {
                        res.unwrap_err();
                    }
                }
                b'A'..=b'Z' => {
                    let v = b - b'A' + 10;
                    if v < r {
                        res.unwrap();
                        tmp.u8_(v);
                        assert!(awi == tmp);
                    } else {
                        res.unwrap_err();
                    }
                }
                b'_' => {
                    res.unwrap();
                    assert!(awi.is_zero());
                }
                _ => {
                    res.unwrap_err();
                }
            }
        }
    }
}

// TODO serde conversion
