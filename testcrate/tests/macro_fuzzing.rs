//! Macro fuzzing tests. Uses code from `build.rs`

#![allow(bad_style)]
#![allow(unused_imports)]

use std::any::Any;

use awint::prelude::*;

/*
#[track_caller]
fn eq(lhs: &dyn Any, rhs: &dyn Any) {
    if let Some(o) = lhs.downcast_ref::<Option<()>>() {
        assert_eq!(o, rhs.downcast_ref::<Option<()>>().unwrap());
    } else if let Some(o) = lhs.downcast_ref::<Option<&dyn AsRef<Bits>>>() {
        match (o, rhs.downcast_ref::<Option<&dyn AsRef<Bits>>>().unwrap()) {
            (None, None) => (),
            (Some(lhs), None) => panic!("lhs != rhs, lhs: {:?}, rhs: None", lhs.as_ref()),
            (None, Some(rhs)) => panic!("lhs != rhs, lhs: None, rhs: {:?}", rhs.as_ref()),
            (Some(lhs), Some(rhs)) => assert_eq!(lhs.as_ref(), rhs.as_ref()),
        }
    } else if let Some(o) = lhs.downcast_ref::<&dyn AsRef<Bits>>() {
        panic!("lhs != rhs, lhs: {:?}, rhs: {:?}", o.as_ref(),
            rhs.downcast_ref::<&dyn AsRef<Bits>>().unwrap().as_ref())
    } else {
        panic!("lhs type unknown")
    }
}

#[test]
fn lkj() {
    eq(&Some(inlawi!(0i1)), &Some(inlawi!(0i1))));
}*/

include!(concat!(env!("OUT_DIR"), "/generated.rs"));
