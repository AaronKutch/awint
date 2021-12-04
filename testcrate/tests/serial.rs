use awint::{extawi, inlawi, ExtAwi, InlAwi, SerdeError::*, FP};

// non-const serialization tests
#[test]
fn string_conversion() {
    let x = "0i1".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 1);
    assert_eq!((x[..]).to_bool(), false);
    let x = "0u1".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 1);
    assert_eq!((x[..]).to_bool(), false);
    let x = "123i64".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 64);
    assert_eq!((x[..]).to_i16(), 123);
    let x = "-123i64".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 64);
    assert_eq!((x[..]).to_i16(), -123);
    let x = "123u64".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 64);
    assert_eq!((x[..]).to_u16(), 123);
    let x = "1u1".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 1);
    assert_eq!((x[..]).to_u16(), 1);
    let x = "-0xf1i16".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 16);
    assert_eq!((x[..]).to_i16(), -0xf1);
    let x = "-0o71i16".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 16);
    assert_eq!((x[..]).to_i16(), -0o71);
    let x = "-07i8".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 8);
    assert_eq!((x[..]).to_i8(), -7);
    let x = "-00008i8".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 8);
    assert_eq!((x[..]).to_i8(), -8);
    let x = "00008u8".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 8);
    assert_eq!((x[..]).to_u8(), 8);
    let x = "010100111001".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 12);
    assert_eq!((x[..]).to_u16(), 1337);
    let x = "0b_0101_0011_1001_u12".parse::<ExtAwi>().unwrap();
    assert_eq!(x.bw(), 12);
    assert_eq!((x[..]).to_u16(), 1337);
    assert!(matches!("".parse::<ExtAwi>(), Err(Empty)));
    assert!(matches!("u".parse::<ExtAwi>(), Err(Empty)));
    assert!(matches!("123i".parse::<ExtAwi>(), Err(Empty)));
    assert!(matches!("123".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("0u0".parse::<ExtAwi>(), Err(ZeroBitwidth)));
    assert!(matches!("i64".parse::<ExtAwi>(), Err(Empty)));
    assert!(matches!("-123i".parse::<ExtAwi>(), Err(Empty)));
    assert!(matches!("-123".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("-i64".parse::<ExtAwi>(), Err(Empty)));
    assert!(matches!("-123u".parse::<ExtAwi>(), Err(Empty)));
    assert!(matches!("-u64".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("-123u8".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("-2i1".parse::<ExtAwi>(), Err(Overflow)));
    assert!(matches!("2u1".parse::<ExtAwi>(), Err(Overflow)));
    assert!(matches!("1i1".parse::<ExtAwi>(), Err(Overflow)));
    assert!(matches!("0xgu8".parse::<ExtAwi>(), Err(InvalidChar)));
    assert!(matches!("0xu8".parse::<ExtAwi>(), Err(Empty)));
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
    let bits = bits_awi.const_as_ref();
    fmt_test!(inl_awi ext_awi bits);
    assert_eq!(format!("{}", inlawi!(0u100)), "0x0_u100");
    assert_eq!(
        format!("{}", inlawi!(0x1_fedcba98_76543210u100)),
        "0x1_fedcba98_76543210_u100"
    );

    let fpbits = FP::new(true, inlawi!(-0xabcd1234i36), 16).unwrap();
    assert_eq!(format!("{:x}", fpbits), "-0xabcd.1234_i36f16");
    assert_eq!(format!("{:X}", fpbits), "-0xABCD.1234_i36f16");
    assert_eq!(format!("{:o}", fpbits), "-0o125715.4432_i36f16");
    assert_eq!(format!("{:b}", fpbits), "-0b1010101111001101.10010001101_i36f16");
    assert_eq!(format!("{:?}", fpbits), "-43981.7111_i36f16");
    assert_eq!(format!("{}", fpbits), "-43981.7111_i36f16");

    let fpbits = FP::new(true, inlawi!(1u16), -16).unwrap();
    assert_eq!(format!("{}", fpbits), "");
}

// TODO serde conversion
