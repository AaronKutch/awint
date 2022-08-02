#![feature(test)]

extern crate test;
use awint::dag_prelude::*;
use awint_dag::{common::Lineage, lowering::Dag};
use test::Bencher;
use triple_arena::ptr_struct;

ptr_struct!(P0; P1; P2);

#[bench]
fn lower_funnel(bencher: &mut Bencher) {
    let mut out = inlawi!(0u32);
    let rhs = inlawi!(opaque: ..64);
    let s = inlawi!(opaque: ..5);
    out.funnel(&rhs, &s).unwrap();

    let (mut dag, res) = Dag::<P0>::new(&[out.state()], &[out.state()]);
    res.unwrap();
    bencher.iter(|| {
        dag.lower().unwrap();
    })
}
