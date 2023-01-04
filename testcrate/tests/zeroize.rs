use awint::awi::*;
use zeroize::Zeroize;

#[test]
fn zeroize() {
    let mut x = inlawi!(0xfedcba9876543210u100);
    let y: &mut Bits = &mut x;
    y.zeroize();
    assert!(x.is_zero());
    let mut x = extawi!(0xfedcba9876543210u100);
    x.zeroize();
    assert!(x.is_zero());
    let mut x = inlawi!(0xfedcba9876543210u100);
    x.zeroize();
    assert!(x.is_zero());
}
