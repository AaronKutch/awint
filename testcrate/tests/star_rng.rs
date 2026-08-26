use awint::awi::*;
use rand_core::Rng;
use star_rng::StarRng;

// downstream crates will rely on a deterministic behavior
#[test]
#[allow(clippy::bool_assert_comparison)]
fn star_rng_test_vectors() {
    let mut rng = StarRng::new(0);
    let mut dst = [0; 65];
    rng.fill_bytes(&mut dst);
    assert_eq!(dst, [
        93, 4, 201, 222, 117, 157, 8, 154, 98, 211, 119, 171, 5, 100, 225, 195, 218, 168, 149, 92,
        86, 160, 222, 96, 64, 81, 90, 194, 20, 6, 41, 164, 175, 37, 5, 158, 185, 55, 61, 149, 119,
        59, 151, 202, 87, 132, 44, 54, 8, 210, 248, 126, 178, 2, 132, 201, 166, 209, 126, 231, 27,
        64, 168, 234, 98
    ]);

    assert_eq!(rng.index(1 << 15).unwrap(), 27303);
    assert_eq!(*rng.index_slice(&dst).unwrap(), 5);
    assert_eq!(*rng.index_slice_mut(&mut dst).unwrap(), 231);
    assert_eq!(rng.next_bool(), false);
    assert_eq!(rng.next_u8(), 60);
    assert_eq!(rng.next_u16(), 55860);
    assert_eq!(rng.next_u32(), 1142491458);
    assert_eq!(rng.next_u64(), 7750222263744231949);
    assert_eq!(rng.next_u128(), 107347936578185361932231794775361051828);
    assert_eq!(rng.out_of_4(1), false);
    assert_eq!(rng.out_of_8(3), true);
    assert_eq!(rng.out_of_16(7), true);
    assert_eq!(rng.out_of_32(20), false);
    assert_eq!(rng.out_of_64(31), true);
    assert_eq!(rng.out_of_128(50), false);
    assert_eq!(rng.out_of_256(100), false);
    assert_eq!(rng.uniform_u8(u8::MAX / 8), 31);
    assert_eq!(rng.uniform_u16(u16::MAX / 8), 2436);
    assert_eq!(rng.uniform_u32(u32::MAX / 8), 414907957);
    assert_eq!(rng.uniform_u64(u64::MAX / 8), 1339345442184903593);
    assert_eq!(
        rng.uniform_u128(u128::MAX / 8),
        13939501709697904127631821811304380035
    );

    assert_eq!(rng.bits_consumed(), 1094);

    let mut bits = inlawi!(umax: ..70);
    bits.star_rng_(&mut rng);
    assert_eq!(bits, inlawi!(0xf_3d737c42_e7a9e29d_u70));
    bits.rand_(&mut rng);
    assert_eq!(bits, inlawi!(0x5_d8de04d7_e0615878_u70));
    bits.star_rng_width_(&mut rng, 40).unwrap();
    assert_eq!(bits, inlawi!(0x4f_d41b7a25_u70));
    bits.star_rng_linear_fuzz_step_(&mut rng);
    assert_eq!(bits, inlawi!(0x1f_ffffffb0_2be485dd_u70));
    bits.star_rng_linear_fuzz_step_(&mut rng);
    assert_eq!(bits, inlawi!(0x1f_ffffffff_ffe485dd_u70));
    bits.star_rng_linear_fuzz_step_(&mut rng);
    assert_eq!(bits, inlawi!(0x7_ffe48580_u70));
}

fn rand_choice(
    metarng: &mut rand_xoshiro::Xoshiro128StarStar,
    rng: &mut StarRng,
    mut bits: &mut awint::Bits,
    actions: &mut u64,
) {
    use std::num::NonZeroUsize;

    use awint::awi::*;

    let mut used = 0;
    loop {
        let remaining = bits.bw() - used;
        if remaining == 0 {
            break;
        }
        if remaining < 192 {
            // need to fill up without encountering a potential overflow case
            let mut tmp = Awi::zero(NonZeroUsize::new(remaining).unwrap());
            tmp.star_rng_(rng);
            cc!(tmp, ..; bits).unwrap();
            break;
        }
        match metarng.next_u32() % 8 {
            0 => {
                cc!(InlAwi::from_bool(rng.next_bool()); bits[used]).unwrap();
                used += 1;
            }
            1 => {
                cc!(InlAwi::from_u8(rng.next_u8()); bits[used..(used+8)]).unwrap();
                used += 8;
            }
            2 => {
                cc!(InlAwi::from_u16(rng.next_u16()); bits[used..(used+16)]).unwrap();
                used += 16;
            }
            3 => {
                cc!(InlAwi::from_u32(rng.next_u32()); bits[used..(used+32)]).unwrap();
                used += 32;
            }
            4 => {
                cc!(InlAwi::from_u64(rng.next_u64()); bits[used..(used+64)]).unwrap();
                used += 64;
            }
            5 => {
                cc!(InlAwi::from_u128(rng.next_u128()); bits[used..(used+128)]).unwrap();
                used += 128;
            }
            6 => {
                let w = NonZeroUsize::new((metarng.next_u32() % 192) as usize + 1).unwrap();
                let mut tmp = Awi::zero(w);
                tmp.star_rng_(rng);
                cc!(tmp; bits[used..(used+w.get())]).unwrap();
                used += w.get();
            }
            7 => {
                let w = NonZeroUsize::new((metarng.next_u32() % 192) as usize + 1).unwrap();
                let mut tmp = Awi::zero(w);
                let width = (metarng.next_u32() as usize) % w.get();
                tmp.star_rng_width_(rng, width).unwrap();
                cc!(tmp[..width]; bits[used..(used+width)]).unwrap();
                used += width;
            }
            _ => unreachable!(),
        }
        *actions += 1;
    }
}

#[test]
fn star_rng_rand_support() {
    use awint::awi::*;

    const N: usize = 1 << 16;
    let mut metarng =
        <rand_xoshiro::Xoshiro128StarStar as rand_core::SeedableRng>::seed_from_u64(1);
    let mut rng0 = StarRng::new(0);
    let mut rng1 = StarRng::new(0);
    let mut bits0 = Awi::zero(bw(N));
    let mut bits1 = Awi::zero(bw(N));
    let mut actions = 0;
    rand_choice(&mut metarng, &mut rng0, &mut bits0, &mut actions);
    assert_eq!(actions, 1273);
    actions = 0;
    // the `metarng` is different and will fill `bits1` in a different way, but the
    // overall result should be the same since the buffering is bitwise and `rng0`
    // and `rng1` started with the same bits
    rand_choice(&mut metarng, &mut rng1, &mut bits1, &mut actions);
    assert_eq!(actions, 1338);
    assert_eq!(bits0, bits1);

    // just to make sure there are not panics
    let mut x = awi!(0u7);
    for _ in 0..100 {
        x.star_rng_linear_fuzz_step_(&mut rng0);
    }
}
