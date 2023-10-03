use std::num::NonZeroUsize;

use awint::{bw, Awi, Bits, ExtAwi};
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

const N: (u64, u64) = (10_000, 9);

#[test]
fn awi_struct() {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut rng1 = Xoshiro128StarStar::seed_from_u64(0);

    let mut next_nzbw = || NonZeroUsize::new(((rng1.next_u32() % 259) + 1) as usize).unwrap();

    let mut iter_max = 0;

    let mut x0 = Awi::zero(bw(1));
    let mut x1 = ExtAwi::zero(bw(1));
    for _ in 0..N.0 {
        assert_eq!(Awi::nzbw(&x0), Bits::nzbw(&x0));
        assert_eq!(Awi::bw(&x0), Bits::bw(&x0));
        assert!(x0.capacity() >= x0.nzbw());
        assert_eq!(x0.as_ref(), x1.as_ref());
        match rng.next_u32() % 16 {
            0 => {
                let w = next_nzbw();
                x0 = Awi::zero(w);
                x1 = ExtAwi::zero(w);
            }
            1 => {
                let w = next_nzbw();
                x0 = Awi::umax(w);
                x1 = ExtAwi::umax(w);
            }
            2 => {
                let w = next_nzbw();
                x0 = Awi::imax(w);
                x1 = ExtAwi::imax(w);
            }
            3 => {
                let w = next_nzbw();
                x0 = Awi::imin(w);
                x1 = ExtAwi::imin(w);
            }
            4 => {
                let w = next_nzbw();
                x0 = Awi::uone(w);
                x1 = ExtAwi::uone(w);
            }
            5 => {
                let w = next_nzbw();
                x0 = Awi::zero_with_capacity(w, next_nzbw());
                x1 = ExtAwi::zero(w);
            }
            6 => {
                let w = next_nzbw();
                x0 = Awi::umax_with_capacity(w, next_nzbw());
                x1 = ExtAwi::umax(w);
            }
            7 => {
                let w = next_nzbw();
                x0 = Awi::imax_with_capacity(w, next_nzbw());
                x1 = ExtAwi::imax(w);
            }
            8 => {
                let w = next_nzbw();
                x0 = Awi::imin_with_capacity(w, next_nzbw());
                x1 = ExtAwi::imin(w);
            }
            9 => {
                let w = next_nzbw();
                x0 = Awi::uone_with_capacity(w, next_nzbw());
                x1 = ExtAwi::uone(w);
            }
            10 => {
                x1.rand_(&mut rng).unwrap();
                x0 = Awi::from_bits(&x1);
            }
            11 => {
                x1.rand_(&mut rng).unwrap();
                x0 = Awi::from_bits_with_capacity(&x1, next_nzbw());
            }
            12 => {
                let cap0 = x0.capacity();
                let additional = ((rng.next_u32() % 10) * 10) as usize;
                x0.reserve(additional);
                assert!(x0.capacity().get() >= (cap0.get() + additional));
            }
            13 => {
                let new_cap = next_nzbw();
                x0.shrink_to(new_cap);
                assert!(x0.capacity() >= new_cap);
            }
            14 => {
                x0.shrink_to_fit();
            }
            15 => {
                iter_max += 1;
            }
            _ => unreachable!(),
        }
    }
    assert_eq!(iter_max, N.1);
}
