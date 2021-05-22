use awint::{inlawi, ExtAwi, InlAwi, SerdeError::*};

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

#[test]
fn debug_strings() {
    let awi = inlawi!(0xfedcba9876543210u100);
    assert_eq!(format!("{:?}", awi), "0xfedcba98_76543210_u100");
    assert_eq!(
        format!("{:?}", awi.const_as_ref()),
        "0xfedcba98_76543210_u100"
    );
    assert_eq!(
        format!("{:?}", ExtAwi::from(awi)),
        "0xfedcba98_76543210_u100"
    );
    assert_eq!(
        format!("{:?}", inlawi!(0x1_fedcba98_76543210u100)),
        "0x1_fedcba98_76543210_u100"
    );
    assert_eq!(format!("{:?}", inlawi!(0u100)), "0x0_u100");
}

// TODO serde conversion
