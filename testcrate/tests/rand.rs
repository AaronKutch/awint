use awint::prelude::*;
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};

#[test]
fn rand() {
    // note: mirror changes of this example to the doctest for the
    // `rand_assign_using` function
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut awi = inlawi_zero!(100);
    awi.const_as_mut().rand_assign_using(&mut rng).unwrap();
    assert_eq!(awi, inlawi!(0x5ab77d3629a089d75dec9045du100));
    awi.const_as_mut().rand_assign_using(&mut rng).unwrap();
    assert_eq!(awi, inlawi!(0x4c25a514060dea0565c95a8dau100));
}

struct Xoshiro {
    state: inlawi_ty!(128),
}

impl Xoshiro {
    pub fn from_seed(seed: [u8; 16]) -> Self {
        let mut awi = inlawi!(0u128);
        let mut tmp_awi = inlawi!(0u8);
        let tmp = tmp_awi.const_as_mut();
        for (i, s) in seed.iter().enumerate() {
            tmp.u8_assign(*s);
            cc!(tmp; awi[(i * 8)..((i + 1) * 8)]).unwrap();
        }
        Self { state: awi }
    }

    pub fn next_u32(&mut self) -> inlawi_ty!(32) {
        let (mut awi0, mut awi1, mut awi2, mut awi3) = (
            inlawi!(self.state[..32]).unwrap(),
            inlawi!(self.state[32..64]).unwrap(),
            inlawi!(self.state[64..96]).unwrap(),
            inlawi!(self.state[96..128]).unwrap(),
        );
        let s0 = awi0.const_as_mut();
        let s1 = awi1.const_as_mut();
        let s2 = awi2.const_as_mut();
        let s3 = awi3.const_as_mut();
        let mut result = inlawi!(s1; ..32).unwrap();
        let r = result.const_as_mut();
        let mut awi_tmp = inlawi!(s1; ..32).unwrap();
        let tmp = awi_tmp.const_as_mut();

        // result = s1.wrapping_mul(5).rotate_left(7).wrapping_mul(9)

        // multiply by 5
        r.shl_assign(2).unwrap();
        r.add_assign(tmp).unwrap();

        r.rotl_assign(7).unwrap();

        // multiply by 9
        cc!(r; tmp).unwrap();
        r.shl_assign(3).unwrap();
        r.add_assign(tmp).unwrap();

        //let t = s1 << 9;
        //s2 ^= s0;
        //s3 ^= s1;
        //s1 ^= s2;
        //s0 ^= s3;
        //s2 ^= t;
        //s3 = s3.rotate_left(11);

        cc!(s1; tmp).unwrap();
        tmp.shl_assign(9).unwrap();
        s2.xor_assign(s0).unwrap();
        s3.xor_assign(s1).unwrap();
        s1.xor_assign(s2).unwrap();
        s0.xor_assign(s3).unwrap();
        s2.xor_assign(tmp).unwrap();
        s3.rotl_assign(11).unwrap();
        cc!(s0; self.state[..32]).unwrap();
        cc!(s1; self.state[32..64]).unwrap();
        cc!(s2; self.state[64..96]).unwrap();
        cc!(s3; self.state[96..128]).unwrap();

        result
    }
}

// This doesn't test `rand_assign_using`, but is more insurance against
// something breaking
#[test]
fn rand_example() {
    let mut rng0 = Xoshiro::from_seed([1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]);
    let mut rng1 = Xoshiro128StarStar::from_seed([1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0]);
    assert_eq!(rng0.next_u32().to_u32(), rng1.next_u32());
    assert_eq!(rng0.next_u32().to_u32(), rng1.next_u32());
    assert_eq!(rng0.next_u32().to_u32(), rng1.next_u32());
    assert_eq!(rng0.next_u32().to_u32(), rng1.next_u32());
}
