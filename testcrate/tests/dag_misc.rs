use awint::{
    awi,
    awint_dag::{state::STATE_ARENA, EvalError, Lineage, Op, OpDag, StateEpoch},
    dag,
};

#[test]
fn state_epochs() {
    use awint::dag::u8;
    let state = {
        let _epoch0 = StateEpoch::new();
        let x: &u8 = &7.into();
        // test `Copy` trait
        let y: u8 = *x;
        assert_eq!(STATE_ARENA.with(|f| f.borrow().len()), 1);
        {
            let _epoch1 = StateEpoch::new();
            let mut _z: u8 = 7.into();
            assert_eq!(STATE_ARENA.with(|f| f.borrow().len()), 2);
        }
        assert_eq!(STATE_ARENA.with(|f| f.borrow().len()), 1);
        {
            let _epoch2 = StateEpoch::new();
            let mut _w: u8 = 7.into();
            assert_eq!(STATE_ARENA.with(|f| f.borrow().len()), 2);
        }
        assert_eq!(STATE_ARENA.with(|f| f.borrow().len()), 1);
        let state = y.state();
        assert!(state.cloned_state().is_some());
        state
    };
    assert!(state.cloned_state().is_none());
    assert!(STATE_ARENA.with(|f| f.borrow().is_empty()))
}

#[test]
#[should_panic]
fn state_epoch_fail() {
    let epoch0 = StateEpoch::new();
    let epoch1 = StateEpoch::new();
    drop(epoch0);
    drop(epoch1);
}

#[test]
fn dag_assertions() {
    use awint::dag::*;
    use dag::{assert, assert_eq, assert_ne};
    let epoch0 = StateEpoch::new();
    let x = inlawi!(13u8);
    let y = inlawi!(13u8);
    let z = inlawi!(99u8);
    let is_true = x.lsb();
    assert!(true);
    assert!(is_true);
    assert_eq!(x, y);
    assert_ne!(x, z);
    core::assert_eq!(epoch0.assertions().bits.len(), 4);
    let (mut graph, res) = OpDag::from_epoch(&epoch0);
    res.unwrap();
    graph.eval_all().unwrap();
    graph.assert_assertions().unwrap();
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
    _epoch0: &StateEpoch,
) {
    use awint::dag::*;
    use dag::assert;

    let [x0, x1, x2, x3, x4] = x;

    // TODO `mul_`, `neg_add_`, `usize_or_`, `short_udivide_`,
    // `short_udivide_inplace`, `range_and`?

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

    // x0.short_cin_mul(0, 0);

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

    // x0.usize_or_(123, 60);

    // division by zero and differing size
    x4.zero_();
    assert!(Bits::udivide(x1, x2, x3, x4).is_none());
    x3.umax_();
    assert!(Bits::udivide(x0, x1, x2, x3).is_none());
    x4.zero_();
    assert!(Bits::idivide(x1, x2, x3, x4).is_none());
    x3.umax_();
    assert!(Bits::idivide(x0, x1, x2, x3).is_none());
    // x1.umax_();
    // assert!(x4.short_udivide_(x1, 0).is_none());
    // x0.umax_();
    // assert!(x4.short_udivide_(x0, 1).is_none());
    // assert!(x4.short_udivide_inplace_(0).is_none());

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

    let epoch0 = StateEpoch::new();

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

    let x5 = inlawi!(opaque: ..128);
    let x6 = inlawi!(opaque: ..192);
    let x7 = inlawi!(opaque: ..192);
    let x8 = inlawi!(opaque: ..192);
    let x9 = inlawi!(opaque: ..192);
    // clone so that we can replace the opaques later
    let y5 = &mut x5.clone();
    let y6 = &mut x6.clone();
    let y7 = &mut x7.clone();
    let y8 = &mut x8.clone();
    let y9 = &mut x9.clone();
    let s2 = inlawi!(opaque: ..64).to_usize();
    let s3 = inlawi!(opaque: ..64).to_usize();
    dag_bits_functions_internal([y5, y6, y7, y8, y9], s2, s3, &epoch0);

    let num_assertions = 170;
    let eq = epoch0.assertions().bits.len() == num_assertions;
    if !eq {
        println!(
            "number of assertions ({}) is not as expected",
            epoch0.assertions().bits.len()
        );
    }
    let (mut graph, res) = OpDag::from_epoch(&epoch0);
    res.unwrap();
    let x5 = graph.note_pstate(x5.state()).unwrap();
    let x6 = graph.note_pstate(x6.state()).unwrap();
    let x7 = graph.note_pstate(x7.state()).unwrap();
    let x8 = graph.note_pstate(x8.state()).unwrap();
    let x9 = graph.note_pstate(x9.state()).unwrap();
    let s2 = graph.note_pstate(s2.state()).unwrap();
    let s3 = graph.note_pstate(s3.state()).unwrap();

    {
        use awi::{assert_eq, *};
        // fix the opaques
        graph.pnote_get_mut_node(x5).unwrap().op = Op::Literal(extawi!(zero: ..128));
        graph.pnote_get_mut_node(x6).unwrap().op = Op::Literal(extawi!(umax: ..192));
        graph.pnote_get_mut_node(x7).unwrap().op = Op::Literal(extawi!(imax: ..192));
        graph.pnote_get_mut_node(x8).unwrap().op = Op::Literal(extawi!(imin: ..192));
        graph.pnote_get_mut_node(x9).unwrap().op = Op::Literal(extawi!(uone: ..192));
        graph.pnote_get_mut_node(s2).unwrap().op =
            Op::Literal(ExtAwi::from_usize(extawi!(127u64).to_usize()));
        graph.pnote_get_mut_node(s3).unwrap().op =
            Op::Literal(ExtAwi::from_usize(extawi!(128u64).to_usize()));

        assert_eq!(graph.assertions.len(), num_assertions);
        graph.eval_all().unwrap();
        assert_eq!(graph.assertions.len(), 0);
        graph.assert_assertions().unwrap();
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
    stuff::test_option_try(7.into()).unwrap();
    stuff::test_result_try(7.into()).unwrap();

    let epoch0 = StateEpoch::new();
    let s = inlawi!(opaque: ..64).to_usize();

    let _ = stuff::test_option_try(s);
    let _ = stuff::test_result_try(s);
    // make sure it is happening at the `Try` point
    std::assert_eq!(epoch0.assertions().bits.len(), 2);
    Option::some_at_dagtime((), false.into()).unwrap();
    Option::<()>::none_at_dagtime(false.into())
        .ok_or(())
        .unwrap_err();
    Result::<(), &str>::ok_at_dagtime((), false.into()).unwrap();
    Result::<&str, ()>::err_at_dagtime((), false.into()).unwrap_err();
    std::assert_eq!(epoch0.assertions().bits.len(), 6);

    let (mut graph, res) = OpDag::from_epoch(&epoch0);
    res.unwrap();
    let s = graph.note_pstate(s.state()).unwrap();
    {
        use awi::{assert, *};
        // fix the opaques
        graph.pnote_get_mut_node(s).unwrap().op = Op::Literal(extawi!(8u64));

        let res = graph.eval_all();
        assert!(matches!(res, Err(EvalError::AssertionFailure(_))));
        for p_note in &graph.assertions {
            use awi::*;
            let p = graph.note_arena[p_note];
            std::assert!(graph.lit(p) == inlawi!(0).as_ref());
        }
        graph.assert_assertions().unwrap_err();
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
fn dag_size() {
    use std::mem;

    use awint::awint_dag::PState;

    #[cfg(debug_assertions)]
    {
        assert_eq!(mem::size_of::<Op<PState>>(), 88);
    }
    #[cfg(not(debug_assertions))]
    {
        assert_eq!(mem::size_of::<Op<PState>>(), 48);
    }
}
