#![cfg(feature = "serde_support")]

use awint::{inlawi, inlawi_ty, InlAwi};

#[test]
fn serde() {
    let awi = inlawi!(0xfedcba9876543210u100);
    assert_eq!(
        ron::to_string(&awi).unwrap(),
        "(bw:100,bits:\"fedcba9876543210\")"
    );

    let awi0 = inlawi!(0xfedcba9876543210u100);
    let awi1: inlawi_ty!(100) = ron::from_str("(bw:100,bits:\"fedcba9876543210\")").unwrap();
    assert_eq!(awi0, awi1);
}
