use awint::{
    awi,
    awint_dag::{
        epoch::{_get_epoch_callback, _get_epoch_gen, _get_epoch_stack, _unregistered_callback},
        Op,
    },
    dag, inlawi_ty,
};
use starlight::{Epoch, LazyAwi};

// FIXME we ultimately need to move any thing that isn't directly mimicking
// types to `starlight`, including the meta lowering since that will have
// specialization anyway

// these tests do not involve the share feature of `starlight` `Epoch`s

#[test]
fn state_epochs() {
    use awint::dag::u8;
    assert_eq!(_get_epoch_gen().get(), 2);
    assert!(_get_epoch_stack().is_empty());
    assert_eq!(_get_epoch_callback(), _unregistered_callback());
    {
        let _epoch0 = Epoch::new();
        assert_eq!(_get_epoch_gen().get(), 3);
        assert_eq!(_get_epoch_stack().len(), 1);
        assert_eq!(_get_epoch_callback(), starlight::epoch::_callback());
        let x: &u8 = &7.into();
        // test `Copy` trait
        let _y: u8 = *x;
        starlight::epoch::no_recursive_current_epoch(|a| {
            assert_eq!(a.epoch_data.borrow().ensemble.stator.states.len(), 1)
        });
        {
            let _epoch1 = Epoch::new();
            assert_eq!(_get_epoch_gen().get(), 4);
            assert_eq!(_get_epoch_stack().len(), 2);
            let mut _z: u8 = 7.into();
            starlight::epoch::no_recursive_current_epoch(|a| {
                assert_eq!(a.epoch_data.borrow().ensemble.stator.states.len(), 1)
            });
        }
        assert_eq!(_get_epoch_stack().len(), 1);
        starlight::epoch::no_recursive_current_epoch(|a| {
            assert_eq!(a.epoch_data.borrow().ensemble.stator.states.len(), 1)
        });
        {
            let _epoch2 = Epoch::new();
            assert_eq!(_get_epoch_gen().get(), 5);
            assert_eq!(_get_epoch_stack().len(), 2);
            let mut _w: u8 = 7.into();
            starlight::epoch::no_recursive_current_epoch(|a| {
                assert_eq!(a.epoch_data.borrow().ensemble.stator.states.len(), 1)
            });
        }
        assert_eq!(_get_epoch_stack().len(), 1);
        starlight::epoch::no_recursive_current_epoch(|a| {
            assert_eq!(a.epoch_data.borrow().ensemble.stator.states.len(), 1)
        });
    };
    assert!(_get_epoch_stack().is_empty());
    assert_eq!(_get_epoch_callback(), _unregistered_callback());
    assert!(starlight::epoch::get_current_epoch().is_none());
}

#[test]
#[should_panic]
fn state_epoch_unregistered0() {
    use dag::*;
    let _x = ExtAwi::zero(bw(1));
}

#[test]
#[should_panic]
fn state_epoch_unregistered1() {
    use dag::*;
    let _x: u8 = 7.into();
}

#[test]
#[should_panic]
fn state_epoch_unregistered2() {
    use dag::*;
    let epoch0 = Epoch::new();
    drop(epoch0);
    let _x: inlawi_ty!(1) = InlAwi::zero();
}

#[test]
#[should_panic]
fn state_epoch_fail() {
    let epoch0 = Epoch::new();
    let epoch1 = Epoch::new();
    drop(epoch0);
    drop(epoch1);
}

#[test]
#[should_panic]
fn dag_assert_eq_fail() {
    use awint::dag::*;
    let epoch0 = Epoch::new();
    let x = extawi!(opaque: ..7);
    let y = extawi!(opaque: ..8);
    dag::assert_eq!(x, y);
    drop(epoch0);
}

#[test]
#[should_panic]
fn dag_assert_ne_fail() {
    use awint::dag::*;
    let epoch0 = Epoch::new();
    let x = extawi!(opaque: ..7);
    let y = extawi!(opaque: ..8);
    dag::assert_ne!(x, y);
    drop(epoch0);
}

#[test]
#[should_panic]
fn dag_assert_eq_fail2() {
    use awint::dag::*;
    let epoch0 = Epoch::new();
    let x = extawi!(13u8);
    let y = extawi!(99u8);
    dag::assert_eq!(x, y);
    drop(epoch0);
}

#[test]
#[should_panic]
fn dag_assert_ne_fail2() {
    use awint::dag::*;
    let epoch0 = Epoch::new();
    let x = extawi!(13u8);
    let y = extawi!(13u8);
    dag::assert_ne!(x, y);
    drop(epoch0);
}

#[test]
fn dag_assertions() {
    use awint::dag::*;
    use dag::{assert, assert_eq, assert_ne};
    let epoch0 = Epoch::new();
    let x = inlawi!(13u8);
    let y = inlawi!(13u8);
    let z = inlawi!(99u8);
    let is_true = x.lsb();
    assert!(true);
    assert!(is_true);
    assert_eq!(x, y);
    assert_ne!(x, z);
    // check that optimizing away is working
    core::assert_eq!(epoch0.assertions().bits.len(), 0);
    let mut lazy_x = LazyAwi::opaque(bw(8));
    let mut lazy_y = LazyAwi::opaque(bw(8));
    let mut lazy_z = LazyAwi::opaque(bw(8));
    let x = &lazy_x;
    let y = &lazy_y;
    let z = &lazy_z;
    let is_true = x.lsb();
    assert!(is_true);
    assert_eq!(x, y);
    assert_ne!(x, z);
    core::assert_eq!(epoch0.assertions().bits.len(), 3);
    {
        use awi::*;
        lazy_x.retro_(&awi!(13u8)).unwrap();
        lazy_y.retro_(&awi!(13u8)).unwrap();
        lazy_z.retro_(&awi!(99u8)).unwrap();
        epoch0.assert_assertions().unwrap();
    }
}

macro_rules! test_nonequal_bw {
    (
        $x0:ident, $x1:ident, $s0:ident, $s1:ident;
        $($fn_unary:ident)*;
        $($fn_unary_shift:ident)*;
        $($fn_binary:ident)*
    ) => {
        $(
            let _ = $x0.$fn_unary(); // Just checking that the function exists and is constant
        )*
        $(
            assert!($x0.$fn_unary_shift($s0).is_some());
            assert!($x0.$fn_unary_shift($s1).is_none());
        )*
        $(
            assert!($x0.$fn_binary($x1).is_none());
        )*
    }
}

macro_rules! test_unary_literal {
    (
        $x0:ident;
        $($fn_unary_literal:ident)*
    ) => {
        $(
            $x0.$fn_unary_literal(1);
        )*
    }
}

fn dag_bits_functions_internal(
    x: [&mut dag::Bits; 5],
    s0: dag::usize,
    s1: dag::usize,
    _epoch0: &Epoch,
) {
    use awint::dag::*;
    // TODO https://github.com/rust-lang/rust/issues/109261
    #[allow(unused_imports)]
    use dag::assert;

    let [x0, x1, x2, x3, x4] = x;

    // TODO `mul_`, `neg_add_`, never add any `digit_` dependent functions

    // test the inlawi macros first
    assert!(x0.is_zero());
    assert!(x1.is_umax());
    assert!(x2.is_imax());
    assert!(x3.is_imin());
    assert!(x4.is_uone());

    // miscellanious functions that won't work with the macro

    // assert!(x0.range_and_(0..128).is_some());
    // assert!(x0.range_and_(127..0).is_some());
    // assert!(x0.range_and_(128..128).is_some());
    // assert!(x0.range_and_(129..129).is_none());
    // // we want to test this reversed range specifically
    // assert!(x0.range_and_(129..128).is_none());
    // assert!(x0.range_and_(0..129).is_none());

    assert!(x0.field(0, x1, 0, 128).is_some());
    assert!(x0.field(0, x1, 64, 128).is_some());
    assert!(x0.field(0, x1, 0, 129).is_none());
    assert!(x0.field(1, x1, 0, 128).is_none());
    assert!(x1.field(0, x0, 0, 128).is_some());
    assert!(x1.field(64, x0, 0, 128).is_some());
    assert!(x1.field(0, x0, 0, 129).is_none());
    assert!(x1.field(0, x0, 1, 128).is_none());
    assert!(x0.field(128, x1, 192, 0).is_some());
    assert!(x0.field(129, x1, 192, 0).is_none());
    assert!(x0.field(128, x1, 193, 0).is_none());
    assert!(x0.field_to(0, x1, 128).is_some());
    assert!(x0.field_to(1, x1, 128).is_none());
    assert!(x0.field_to(0, x1, 129).is_none());
    assert!(x0.field_to(128, x1, 0).is_some());
    assert!(x1.field_to(0, x0, 128).is_some());
    assert!(x1.field_to(64, x0, 128).is_some());
    assert!(x1.field_to(0, x0, 129).is_none());
    assert!(x1.field_to(192, x0, 0).is_some());
    assert!(x0.field_from(x1, 0, 128).is_some());
    assert!(x0.field_from(x1, 64, 128).is_some());
    assert!(x0.field_from(x1, 65, 128).is_none());
    assert!(x0.field_from(x1, 0, 129).is_none());
    assert!(x0.field_from(x1, 192, 0).is_some());
    assert!(x1.field_from(x0, 0, 128).is_some());
    assert!(x1.field_from(x0, 65, 128).is_none());
    assert!(x1.field_from(x0, 64, 129).is_none());
    assert!(x1.field_from(x0, 128, 0).is_some());
    assert!(x0.field_width(x1, 0).is_some());
    assert!(x0.field_width(x1, 128).is_some());
    assert!(x0.field_width(x1, 129).is_none());
    assert!(x1.field_width(x0, 0).is_some());
    assert!(x1.field_width(x0, 128).is_some());
    assert!(x1.field_width(x0, 129).is_none());
    assert!(x0.field_bit(0, x1, 0).is_some());
    assert!(x0.field_bit(127, x1, 191).is_some());
    assert!(x0.field_bit(128, x1, 191).is_none());
    assert!(x0.field_bit(127, x1, 192).is_none());

    assert!(x0.lut_(x1, x3).is_none());
    assert!(x0.lut_set(x1, x3).is_none());
    assert!(x0.funnel_(x1, x3).is_none());

    assert!(x0.mul_add_(x1, x2).is_none());
    assert!(x1.mul_add_(x0, x2).is_none());
    assert!(x2.mul_add_(x1, x0).is_none());
    // assert!(x0.mul_(x1, x2).is_none());
    // assert!(x1.mul_(x0, x2).is_none());
    // assert!(x2.mul_(x1, x0).is_none());

    x0.arb_umul_add_(x1, x2);
    x0.arb_imul_add_(x1, x2);

    // tests that assign constants to the vars are moved to the bottom so that
    // `Pass` evaluations are tested correctly

    assert!(x0.get(128).is_none());
    assert!(x0.set(128, false).is_none());

    assert!(x0.mux_(x1, false).is_none());
    assert!(x0.mux_(x1, true).is_none());

    test_nonequal_bw!(
        x0, x1, s0, s1
        ;// functions with signature `fn({ , &, &mut} self) -> ...`
        nzbw
        bw
        zero_
        umax_
        imax_
        imin_
        uone_
        not_
        is_zero
        is_umax
        is_imin
        is_uone
        lsb
        msb
        lz
        tz
        count_ones
        rev_
        to_usize
        to_isize
        to_u8
        to_i8
        to_u16
        to_i16
        to_u32
        to_i32
        to_u64
        to_i64
        to_u128
        to_i128
        to_bool
        abs_
        ;
        shl_
        lshr_
        ashr_
        rotl_
        rotr_
        ;// functions with signature `fn({ , &, &mut} self, rhs: { , &, &mut} Self) -> Option<...>`
        copy_
        or_
        and_
        xor_
        const_eq
        const_ne
        ult
        ule
        ugt
        uge
        ilt
        ile
        igt
        ige
        add_
        sub_
        rsb_
    );

    // assert!(x0.neg_add_(false, x1).is_none());
    assert!(x0.cin_sum_(false, x1, x2).is_none());

    x0.inc_(false);
    x0.dec_(true);
    x0.neg_(false);

    // division by zero and differing size
    x4.zero_();
    assert!(Bits::udivide(x1, x2, x3, x4).is_none());
    x3.umax_();
    assert!(Bits::udivide(x0, x1, x2, x3).is_none());
    x4.zero_();
    assert!(Bits::idivide(x1, x2, x3, x4).is_none());
    x3.umax_();
    assert!(Bits::idivide(x0, x1, x2, x3).is_none());

    x0.bool_(true);
    test_unary_literal!(
        x0;
        // functions with signature `fn({ , &, &mut} self, rhs: integer) -> ...`
        usize_
        isize_
        u8_
        i8_
        u16_
        i16_
        u32_
        i32_
        u64_
        i64_
        u128_
        i128_
    );
}

// Counterpart to `bits_functions` that checks failure cases
#[test]
fn dag_bits_functions() {
    use awint::dag::*;

    let epoch0 = Epoch::new();

    let mut x0 = inlawi!(zero: ..128);
    let mut x1 = inlawi!(umax: ..192);
    let mut x2 = inlawi!(imax: ..192);
    let mut x3 = inlawi!(imin: ..192);
    let mut x4 = inlawi!(uone: ..192);
    let y0 = &mut x0;
    let y1 = &mut x1;
    let y2 = &mut x2;
    let y3 = &mut x3;
    let y4 = &mut x4;
    let s0 = inlawi!(127u64).to_usize();
    let s1 = inlawi!(128u64).to_usize();
    dag_bits_functions_internal([y0, y1, y2, y3, y4], s0, s1, &epoch0);

    awi::assert!(epoch0.assertions().bits.is_empty());

    let mut x5 = LazyAwi::opaque(bw(128));
    let mut x6 = LazyAwi::opaque(bw(192));
    let mut x7 = LazyAwi::opaque(bw(192));
    let mut x8 = LazyAwi::opaque(bw(192));
    let mut x9 = LazyAwi::opaque(bw(192));
    let mut y5 = awi!(x5);
    let mut y6 = awi!(x6);
    let mut y7 = awi!(x7);
    let mut y8 = awi!(x8);
    let mut y9 = awi!(x9);
    let mut s2 = LazyAwi::opaque(bw(64));
    let mut s3 = LazyAwi::opaque(bw(64));
    dag_bits_functions_internal(
        [&mut y5, &mut y6, &mut y7, &mut y8, &mut y9],
        s2.to_usize(),
        s3.to_usize(),
        &epoch0,
    );

    let num_assertions = 15;
    let eq = epoch0.assertions().bits.len() == num_assertions;
    if !eq {
        panic!(
            "number of assertions ({}) is not as expected",
            epoch0.assertions().bits.len()
        );
    }

    {
        use awi::{assert, *};

        x5.retro_(&awi!(zero: ..128)).unwrap();
        x6.retro_(&awi!(umax: ..192)).unwrap();
        x7.retro_(&awi!(imax: ..192)).unwrap();
        x8.retro_(&awi!(imin: ..192)).unwrap();
        x9.retro_(&awi!(uone: ..192)).unwrap();
        s2.retro_(&Awi::from_usize(127)).unwrap();
        s3.retro_(&Awi::from_usize(128)).unwrap();

        // FIXME
        //epoch0.lower().unwrap();
        //assert!(epoch0.assertions().bits.is_empty());
        //epoch0.assert_assertions().unwrap();
        if !eq {
            panic!();
        }
    }
}

mod stuff {
    use super::dag::*;

    pub fn test_option_try(s: usize) -> Option<()> {
        let mut x = inlawi!(0x88u8);
        x.shl_(s)?;
        Some(())
    }

    pub fn test_result_try(s: usize) -> Result<(), &'static str> {
        let mut x = inlawi!(0x88u8);
        x.shl_(s).ok_or("err")?;
        Ok(())
    }
}

#[test]
#[should_panic]
fn dag_option_try_fail() {
    stuff::test_option_try(8.into()).unwrap();
}

#[test]
#[should_panic]
fn dag_result_try_fail() {
    stuff::test_result_try(8.into()).unwrap();
}

#[test]
fn dag_try() {
    use dag::*;

    let epoch1 = Epoch::new();
    stuff::test_option_try(7.into()).unwrap();
    stuff::test_result_try(7.into()).unwrap();
    drop(epoch1);

    let epoch0 = Epoch::new();
    let mut s = LazyAwi::opaque(bw(64));

    let _ = stuff::test_option_try(s.to_usize());
    let _ = stuff::test_result_try(s.to_usize());
    // make sure it is happening at the `Try` point
    std::assert_eq!(epoch0.assertions().bits.len(), 2);
    Option::some_at_dagtime((), false.into()).unwrap();
    Option::<()>::none_at_dagtime(false.into())
        .ok_or(())
        .unwrap_err();
    Result::<(), &str>::ok_at_dagtime((), false.into()).unwrap();
    Result::<&str, ()>::err_at_dagtime((), false.into()).unwrap_err();
    std::assert_eq!(epoch0.assertions().bits.len(), 6);

    {
        use awi::*;

        s.retro_(&awi!(8u64)).unwrap();

        // TODO
        //assert!(matches!(epoch0.assert_assertions(),
        // Err(EvalError::AssertionFailure(_))));
        epoch0.assert_assertions().unwrap_err();
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
fn dag_size() {
    use std::mem;

    use awint::awint_dag::PState;

    #[cfg(not(debug_assertions))]
    {
        assert_eq!(mem::size_of::<Op<PState>>(), 48);
    }
    #[cfg(debug_assertions)]
    {
        assert_eq!(mem::size_of::<Op<PState>>(), 88);
    }
}
