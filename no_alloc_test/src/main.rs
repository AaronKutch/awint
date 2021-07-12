#![no_std]
#![no_main]

extern crate panic_halt;

use awint::prelude::*;
use rand_xoshiro::{rand_core::SeedableRng, Xoshiro128StarStar};
use riscv_minimal_rt::entry;

#[entry]
fn main() -> ! {
    // test that the procedural macros can still use allocation at compile time
    // without the dependency leaking into runtime
    let mut awi0 = inlawi!(12345i20);
    let awi1 = inlawi!(54321i20);
    let x0 = awi0.const_as_mut();
    let x1 = awi1.const_as_ref();
    x0.add_assign(x1).unwrap();
    assert!(x0.is_zero());
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    x0.rand_assign_using(&mut rng).unwrap();
    cc!(x1; x0).unwrap();
    let _ = inlawi_umax!(.., x0; ..100).unwrap();

    // copied some macro tests here to make sure that `ExtAwi` is not brought in
    // through future changes

    // both trailing comma and semicolon
    let _ = inlawi!(0u1,;);
    // basic concatenation
    assert_eq!(inlawi!(0xau4, 0x4321u16, 0x7u4), inlawi!(0xa43217u24));
    assert_eq!(inlawi!(0xau4, 0x4321u32[8..12], 0x7u4), inlawi!(0xa37u12));
    // copy assign
    let a = inlawi!(0xau4);
    let mut awi = <inlawi_ty!(4)>::zero();
    let b = awi.const_as_mut();
    let mut c = inlawi!(0u4);
    cc!(a;b;c).unwrap();
    assert_eq!(a, inlawi!(0xau4));
    assert_eq!(a.const_as_ref(), b);
    assert_eq!(a.const_as_ref(), c.const_as_ref());
    // dynamic ranges
    let x: usize = 8;
    let awi = inlawi!(0u12);
    assert_eq!(
        inlawi!(0x98765_u20[x..(x + awi.bw())]; ..12).unwrap(),
        inlawi!(0x987u12)
    );
    // unbounded fillers
    let mut sink0 = inlawi!(0u44);
    let mut sink1 = inlawi!(0u44);
    let b = inlawi!(0xbbu8);
    let e = inlawi!(0xeeeu12);
    let result = inlawi!(0xabbcffdeeefu44);
    assert_eq!(
        inlawi_umax!(0xau4, b, 0xcu4, .., 0xdu4, e, 0xfu4; sink0; sink1; ..44).unwrap(),
        result
    );
    assert_eq!(sink0, result);
    assert_eq!(sink1, result);
    let mut sink0 = inlawi!(0xf0f0f0f0f0fu44);
    let mut sink1 = inlawi!(0xf0f0f0f0f0fu44);
    let b = inlawi!(0xbbu8);
    let e = inlawi!(0xeeeu12);
    let result = inlawi!(0xabbcf0deeefu44);
    cc!(0xau4, b, 0xcu4, .., 0xdu4, e, 0xfu4; sink0; sink1; ..44).unwrap();
    assert_eq!(sink0, result);
    assert_eq!(sink1, result);
    assert_eq!(inlawi_umax!(..;..9), inlawi_umax!(9));
    let a = inlawi!(0x123u12);
    let b = inlawi!(0x4u4);
    assert_eq!(inlawi!(a, b; ..16).unwrap(), inlawi!(0x1234u16));
    let a = inlawi!(0xau4);
    let mut b = inlawi!(0xbu4);
    let r0 = 0;
    let r1 = 4;
    assert_eq!(inlawi!(a[..r0], b[..r1]; ..4), Some(inlawi!(0xbu4)));
    assert_eq!(cc!(.., a[..r0];.., b[..r0]; ..100), Some(()));
    assert_eq!(cc!(r0..r1), Some(()));
    assert_eq!(inlawi!(0100[2]), inlawi!(1));
    assert_eq!(inlawi!(0100[3]), inlawi!(0));
    let r = 2;
    assert_eq!(inlawi!(0100[r]; ..1).unwrap(), inlawi!(1));
    let _a = inlawi!(0xau4);
    let mut _y = inlawi!(0u16);
    assert_eq!(_y, inlawi!(0x7fafu16));

    panic!("main is not allowed to return")
}
