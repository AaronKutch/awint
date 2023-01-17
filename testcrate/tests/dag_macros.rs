#![allow(clippy::let_unit_value)]

use awint::{
    awint_dag::{OpDag, StateEpoch},
    dag::{assert, assert_eq, *},
};

// just copied from `macros.rs` and all asserts stripped (since the DAG
// currently assumes all operations succeed)
#[test]
fn dag_macros() {
    let epoch0 = StateEpoch::new();
    // both trailing comma and semicolon
    let _ = inlawi!(0u1,;);
    // basic concatenation
    assert_eq!(inlawi!(0xau4, 0x4321u16, 0x7u4), inlawi!(0xa43217u24));
    assert_eq!(inlawi!(0xau4, 0x4321u32[8..12], 0x7u4), inlawi!(0xa37u12));
    // copy assign
    let a = inlawi!(0xau4);
    let mut awi = ExtAwi::zero(bw(4));
    let mut b = awi.const_as_mut();
    let mut c = extawi!(0u4);
    cc!(a;b;c).unwrap();
    assert_eq!(a, inlawi!(0xau4));
    assert_eq!(a.const_as_ref(), b);
    assert_eq!(a.const_as_ref(), c.const_as_ref());
    // dynamic ranges
    let x = 8;
    let awi = ExtAwi::zero(bw(12));
    assert_eq!(
        extawi!(0x98765_u20[x..(x + awi.bw())]).unwrap(),
        extawi!(0x987u12)
    );
    // unbounded fillers
    let mut sink0 = ExtAwi::zero(bw(44));
    let mut sink1 = ExtAwi::zero(bw(44));
    let b = inlawi!(0xbbu8);
    let e = inlawi!(0xeeeu12);
    let result = extawi!(0xabbcffdeeefu44);
    assert_eq!(
        extawi!(umax: 0xau4, b, 0xcu4, .., 0xdu4, e, 0xfu4; sink0; sink1).unwrap(),
        result.clone()
    );
    assert_eq!(sink0, result.clone());
    assert_eq!(sink1, result);
    let mut sink0 = extawi!(0xf0f0f0f0f0fu44);
    let mut sink1 = extawi!(0xf0f0f0f0f0fu44);
    let b = inlawi!(0xbbu8);
    let e = inlawi!(0xeeeu12);
    let result = extawi!(0xabbcf0deeefu44);
    cc!(0xau4, b, 0xcu4, .., 0xdu4, e, 0xfu4; sink0; sink1).unwrap();
    assert_eq!(sink0, result.clone());
    assert_eq!(sink1, result);
    assert_eq!(extawi!(umax: ..;..9), extawi!(umax: ..9));
    let a = inlawi!(0x123u12);
    let b = inlawi!(0x4u4);
    assert_eq!(extawi!(a, b), extawi!(0x1234u16));
    let a = inlawi!(0xau4);
    let mut b = inlawi!(0xbu4);
    let r0 = 0;
    let r1 = 4;
    assert_eq!(extawi!(a[..r0], b[..r1]).unwrap(), extawi!(0xbu4));
    cc!(a[..r0]; b[..r0]).unwrap();
    assert!(extawi!(a[..r0]).is_none());
    cc!(r0..r1).unwrap();
    assert_eq!(inlawi!(0100[2]), inlawi!(1));
    assert_eq!(inlawi!(0100[3]), inlawi!(0));
    let r = 2;
    assert_eq!(extawi!(0100[r]).unwrap(), extawi!(1));
    let a = inlawi!(0xau4);
    let mut y = inlawi!(0u16);
    cc!(imax: .., a, ..4; y).unwrap();
    assert_eq!(y, inlawi!(0x7fafu16));
    // make sure sink -> buffer refreshes between sinks
    let mut a = inlawi!(0xaaaau16);
    let mut b = inlawi!(0xbbbbu16);
    let mut c = inlawi!(0xccccu16);
    let mut d = inlawi!(0xddddu16);
    cc!(
        ..8, 0x1111u16, ..8;
        a, b;
        c, d;
    )
    .unwrap();
    assert_eq!(a, inlawi!(0xaa11u16));
    assert_eq!(b, inlawi!(0x11bbu16));
    assert_eq!(c, inlawi!(0xcc11u16));
    assert_eq!(d, inlawi!(0x11ddu16));
    // check for borrow collisions
    let mut a = inlawi!(0x9876543210u40);
    let b = extawi!(
        a[..=7], a[(a.bw() - 16)..];
        a[(5 * 4)..(9 * 4)], a[..(2 * 4)];
    )
    .unwrap();
    assert_eq!(a, inlawi!(0x9109843276u40));
    assert_eq!(b, extawi!(0x109876_u24));
    let mut a = inlawi!(0x9876543210u40);
    let b = extawi!(
        a[..=0x7], a[(a.bw() - 0b10000)..];
        a[(5 * 4)..(9 * 4)], a[..0o10];
    )
    .unwrap();
    assert_eq!(a, inlawi!(0x9109843276u40));
    assert_eq!(b, extawi!(0x109876_u24));
    let r0 = 3;
    let r1 = 7;
    cc!(0x123u12[r0..r1]).unwrap();
    let e = 2;
    assert_eq!(extawi!(uone:  ..=, ; ..=18, ..e, ..=, ), extawi!(0x1_u21));
    let r = 3;
    let x = inlawi!(0x8_u5);
    let mut y = inlawi!(0u15);
    cc!(imax: 0..=1, 0x0_u1[0..1], x[..=], 0..=r, ..3; y).unwrap();
    assert_eq!(y, inlawi!(0x247fu15));
    let mut x = inlawi!(0xffu8);
    let mut y = inlawi!(0xfu4);
    cc!(uone: ..; .., x; .., y);
    assert_eq!(x, inlawi!(1u8));
    assert_eq!(y, inlawi!(1u4));
    let mut x = extawi!(0u64);
    assert_eq!(extawi!(umax: ..; x), ExtAwi::umax(bw(64)));
    assert_eq!(x, ExtAwi::umax(bw(64)));
    let r0 = 0;
    let r1 = 0;
    let mut x = inlawi!(0u4);
    let mut y = inlawi!(0u8);
    assert!(cc!(zero: ..; .., x[..r0]; .., y[..r1]).is_some());
    assert_eq!(extawi!(imax: ..; .., x; .., y), extawi!(0x7fu8));
    assert_eq!(x, inlawi!(0xfu4));
    assert_eq!(y, inlawi!(0x7fu8));
    let r = 0;
    assert!(extawi!(imin: ..r).is_none());
    let r = 2;
    assert_eq!(extawi!(imin: ..r).unwrap(), extawi!(10));
    assert_eq!(extawi!(imin: ..2), extawi!(10));
    assert_eq!(inlawi!(imin: ..2), inlawi!(10));
    let y = inlawi!(0u8);
    let _: () = cc!(imin: y);
    assert_eq!(y, inlawi!(0u8));
    let _: () = cc!(imin: ..r);
    let (mut op_dag, res) = OpDag::from_epoch(&epoch0);
    res.unwrap();
    op_dag.eval_all().unwrap();
    op_dag.assert_assertions().unwrap();
}
