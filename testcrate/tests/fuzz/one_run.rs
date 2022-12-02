//! Tests that only need to be run once per bitwidth

use awint::Bits;

use crate::fuzz::{check_invariants, eq};

pub fn one_run(array: [&mut Bits; 4]) -> Option<()> {
    let [x2, x3, x4, x5] = array;
    let w = x2.bw();
    x2.zero_();
    x3.zero_();
    x4.zero_();
    x5.zero_();
    check_invariants(x2);
    assert!(x2.is_zero());
    assert!(!x2.lsb());
    assert!(!x2.msb());
    x2.not_();
    x3.umax_();
    eq(x2, x3);
    assert!(x2.is_umax());
    assert!(x2.lsb());
    assert!(x2.msb());
    x2.imax_();
    check_invariants(x2);
    assert!(x2.is_imax());
    if w != 1 {
        assert!(x2.lsb());
    } else {
        assert!(!x2.lsb());
    }
    assert!(!x2.msb());
    x2.imin_();
    check_invariants(x2);
    assert!(x2.is_imin());
    if w != 1 {
        assert!(!x2.lsb());
    } else {
        assert!(x2.lsb());
    }
    assert!(x2.msb());
    x2.uone_();
    check_invariants(x2);
    assert!(x2.lsb());
    if w != 1 {
        assert!(!x2.msb());
    } else {
        assert!(x2.msb());
    }
    // double check corner case
    x2.imin_();
    x3.umax_();
    Bits::idivide(x4, x5, x2, x3).unwrap();
    assert!(x4.is_imin());
    assert!(x5.is_zero());
    Some(())
}
