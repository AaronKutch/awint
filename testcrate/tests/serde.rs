use awint::{inlawi, inlawi_ty, Bits, InlAwi};

#[test]
fn serde() {
    let awi0 = inlawi!(0xfedcba9876543210u100);
    let s = "(bw:100,bits:\"fedcba9876543210\")";
    assert_eq!(ron::to_string(&awi0).unwrap(), s);

    let awi1: inlawi_ty!(100) = ron::from_str(s).unwrap();
    assert_eq!(awi0, awi1);

    // check that the buffer is not messed up
    let awi0 = inlawi!(1u1);
    let s = "(bw:1,bits:\"1\")";
    assert_eq!(ron::to_string(&awi0).unwrap(), s);

    let awi1: inlawi_ty!(1) = ron::from_str(s).unwrap();
    assert_eq!(awi0, awi1);
}
