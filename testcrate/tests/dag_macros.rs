use awint::{awi, dag_prelude::*};

// just copied from `macros.rs` and all asserts stripped (since the DAG
// currently assumes all operations succeed)
#[test]
fn dag_macros() {
    let _ = inlawi!(0u1,;);
    // basic concatenation
    let _ = inlawi!(0xau4, 0x4321u16, 0x7u4);
    let _ = inlawi!(0xau4, 0x4321u32[8..12], 0x7u4);
    // copy assign
    let a = inlawi!(0xau4);
    let mut awi = ExtAwi::zero(bw(4));
    let mut b = awi.const_as_mut();
    let mut c = extawi!(0u4);
    cc!(a;b;c).unwrap();
    // dynamic ranges
    let x = 8;
    let awi = ExtAwi::zero(bw(12));
    let _ = extawi!(0x98765_u20[x..(x + awi.bw())]).unwrap();
    // unbounded fillers
    let mut sink0 = ExtAwi::zero(bw(44));
    let mut sink1 = ExtAwi::zero(bw(44));
    let b = inlawi!(0xbbu8);
    let e = inlawi!(0xeeeu12);
    let _ = extawi!(0xabbcffdeeefu44);
    let _ = extawi!(umax: 0xau4, b, 0xcu4, .., 0xdu4, e, 0xfu4; sink0; sink1).unwrap();
    let mut sink0 = extawi!(0xf0f0f0f0f0fu44);
    let mut sink1 = extawi!(0xf0f0f0f0f0fu44);
    let b = inlawi!(0xbbu8);
    let e = inlawi!(0xeeeu12);
    let _ = extawi!(0xabbcf0deeefu44);
    cc!(0xau4, b, 0xcu4, .., 0xdu4, e, 0xfu4; sink0; sink1).unwrap();
    let _ = extawi!(umax: ..;..9);
    let _ = extawi!(umax: ..9);
    let a = inlawi!(0x123u12);
    let b = inlawi!(0x4u4);
    let _ = extawi!(a, b);
    let a = inlawi!(0xau4);
    let mut b = inlawi!(0xbu4);
    let r0 = 0;
    let r1 = 4;
    let _ = extawi!(a[..r0], b[..r1]);
    let _ = cc!(a[..r0]; b[..r0]);
    let _ = extawi!(a[..r0]);
    let _ = cc!(r0..r1);
    let _ = inlawi!(0100[2]);
    let _ = inlawi!(0100[3]);
    let r = 2;
    assert!(extawi!(0100[r]).is_some());
    let a = inlawi!(0xau4);
    let mut y = inlawi!(0u16);
    cc!(imax: .., a, ..4; y).unwrap();
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
    // check for borrow collisions
    let mut a = inlawi!(0x9876543210u40);
    let _ = extawi!(
        a[..=7], a[(a.bw() - 16)..];
        a[(5 * 4)..(9 * 4)], a[..(2 * 4)];
    )
    .unwrap();
    let _ = extawi!(
        a[..=0x7], a[(a.bw() - 0b10000)..];
        a[(5 * 4)..(9 * 4)], a[..0o10];
    )
    .unwrap();
    let r0 = 3;
    let r1 = 7;
    let _ = cc!(0x123u12[r0..r1]);
    let e = 2;
    let _ = extawi!(uone:  ..=, ; ..=18, ..e, ..=, );
    let r = 3;
    let x = inlawi!(0x8_u5);
    let mut y = inlawi!(0u15);
    cc!(imax: 0..=1, 0x0_u1[0..1], x[..=], 0..=r, ..3; y).unwrap();
    let mut x = inlawi!(0xffu8);
    let mut y = inlawi!(0xfu4);
    cc!(uone: ..; .., x; .., y);
    let r0 = 0;
    let r1 = 0;
    let mut x = inlawi!(0u4);
    let mut y = inlawi!(0u8);
    let _ = cc!(zero: ..; .., x[..r0]; .., y[..r1]).is_some();
    let _ = extawi!(imax: ..; .., x; .., y);
    let r = 2;
    let y = inlawi!(0u8);
    cc!(imin: y);
    cc!(imin: ..r);
}
