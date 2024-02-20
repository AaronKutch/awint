use awint::{
    awi,
    awint_dag::{
        epoch::{_get_epoch_callback, _get_epoch_gen, _get_epoch_stack, _unregistered_callback},
        Op,
    },
    dag, inlawi_ty,
};

use crate::dag_tests::{Epoch, LazyAwi, _test_callback};

#[test]
fn dag_epochs() {
    use awint::dag::u8;
    assert_eq!(_get_epoch_gen().get(), 2);
    assert!(_get_epoch_stack().is_empty());
    assert_eq!(_get_epoch_callback(), _unregistered_callback());
    {
        let epoch0 = Epoch::new();
        assert_eq!(_get_epoch_gen().get(), 3);
        assert_eq!(_get_epoch_stack().len(), 1);
        assert_eq!(_get_epoch_callback(), _test_callback());
        let x: &u8 = &7.into();
        // test `Copy` trait
        let _y: u8 = *x;
        epoch0.get_states(|states| assert_eq!(states.len(), 1));
        {
            let epoch1 = Epoch::new();
            assert_eq!(_get_epoch_gen().get(), 4);
            assert_eq!(_get_epoch_stack().len(), 2);
            let mut _z: u8 = 7.into();
            epoch1.get_states(|states| assert_eq!(states.len(), 1));
        }
        assert_eq!(_get_epoch_stack().len(), 1);
        epoch0.get_states(|states| assert_eq!(states.len(), 1));
        {
            let epoch2 = Epoch::new();
            assert_eq!(_get_epoch_gen().get(), 5);
            assert_eq!(_get_epoch_stack().len(), 2);
            let mut _w: u8 = 7.into();
            epoch2.get_states(|states| assert_eq!(states.len(), 1));
        }
        assert_eq!(_get_epoch_stack().len(), 1);
        epoch0.get_states(|states| assert_eq!(states.len(), 1));
    };
    assert!(_get_epoch_stack().is_empty());
    assert_eq!(_get_epoch_callback(), _unregistered_callback());
}

#[test]
#[should_panic]
fn dag_epoch_unregistered0() {
    use dag::*;
    let _x = ExtAwi::zero(bw(1));
}

#[test]
#[should_panic]
fn dag_epoch_unregistered1() {
    use dag::*;
    let _x: u8 = 7.into();
}

#[test]
#[should_panic]
fn dag_epoch_unregistered2() {
    use dag::*;
    let epoch0 = Epoch::new();
    drop(epoch0);
    let _x: inlawi_ty!(1) = InlAwi::zero();
}

#[test]
#[should_panic]
fn dag_epoch_fail() {
    let epoch0 = Epoch::new();
    let epoch1 = Epoch::new();
    drop(epoch0);
    drop(epoch1);
}

#[test]
#[should_panic]
fn dag_assert_eq_fail() {
    use dag::*;
    let epoch0 = Epoch::new();
    let x = extawi!(opaque: ..7);
    let y = extawi!(opaque: ..8);
    mimick::assert_eq!(x, y);
    drop(epoch0);
}

#[test]
#[should_panic]
fn dag_assert_ne_fail() {
    use dag::*;
    let epoch0 = Epoch::new();
    let x = extawi!(opaque: ..7);
    let y = extawi!(opaque: ..8);
    mimick::assert_ne!(x, y);
    drop(epoch0);
}

#[test]
#[should_panic]
fn dag_assert_eq_fail2() {
    use dag::*;
    let epoch0 = Epoch::new();
    let x = extawi!(13u8);
    let y = extawi!(99u8);
    mimick::assert_eq!(x, y);
    drop(epoch0);
}

#[test]
#[should_panic]
fn dag_assert_ne_fail2() {
    use dag::*;
    let epoch0 = Epoch::new();
    let x = extawi!(13u8);
    let y = extawi!(13u8);
    mimick::assert_ne!(x, y);
    drop(epoch0);
}

#[test]
fn dag_assertions() {
    use dag::*;
    let epoch0 = Epoch::new();
    let x = inlawi!(13u8);
    let y = inlawi!(13u8);
    let z = inlawi!(99u8);
    let is_true = x.lsb();
    mimick::assert!(true);
    mimick::assert!(is_true);
    mimick::assert_eq!(x, y);
    mimick::assert_ne!(x, z);
    // check that optimizing away is working
    assert_eq!(epoch0.assertions().len(), 0);
    let lazy_x = LazyAwi::opaque(bw(8));
    let lazy_y = LazyAwi::opaque(bw(8));
    let lazy_z = LazyAwi::opaque(bw(8));
    let x = &lazy_x;
    let y = &lazy_y;
    let z = &lazy_z;
    let is_true = x.lsb();
    mimick::assert!(is_true);
    mimick::assert_eq!(x, y);
    mimick::assert_ne!(x, z);
    assert_eq!(epoch0.assertions().len(), 3);
    {
        use awi::*;
        lazy_x.retro_(&awi!(13u8)).unwrap();
        lazy_y.retro_(&awi!(13u8)).unwrap();
        lazy_z.retro_(&awi!(99u8)).unwrap();
        epoch0.assert_assertions().unwrap();
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
    let s = LazyAwi::opaque(bw(64));

    let _ = stuff::test_option_try(s.to_usize());
    let _ = stuff::test_result_try(s.to_usize());
    // make sure it is happening at the `Try` point
    assert_eq!(epoch0.assertions().len(), 2);
    Option::some_at_dagtime((), false.into()).unwrap();
    Option::<()>::none_at_dagtime(false.into())
        .ok_or(())
        .unwrap_err();
    Result::<(), &str>::ok_at_dagtime((), false.into()).unwrap();
    Result::<&str, ()>::err_at_dagtime((), false.into()).unwrap_err();
    assert_eq!(epoch0.assertions().len(), 6);

    {
        use awi::*;

        s.retro_(&awi!(8u64)).unwrap();

        assert!(epoch0.assert_assertions().is_err());
    }
}

#[cfg(target_pointer_width = "64")]
#[test]
#[ignore]
fn dag_size() {
    use std::mem;

    use awint::awint_dag::PState;

    #[cfg(not(debug_assertions))]
    {
        assert_eq!(mem::size_of::<Op<PState>>(), 72);
    }
    #[cfg(debug_assertions)]
    {
        assert_eq!(mem::size_of::<Op<PState>>(), 104);
    }
}
