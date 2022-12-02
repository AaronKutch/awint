#![feature(test)]

extern crate test;
use awint::{
    awint_dag::{Lineage, OpDag},
    dag_prelude::*,
};
use test::Bencher;
use triple_arena::ptr_struct;

ptr_struct!(P0; P1; P2);

#[bench]
fn lower_funnel(bencher: &mut Bencher) {
    bencher.iter(|| {
        let mut out = inlawi!(0u32);
        let rhs = inlawi!(opaque: ..64);
        let s = inlawi!(opaque: ..5);
        out.funnel(&rhs, &s).unwrap();

        let (mut op_dag, res) = OpDag::new(&[out.state()], &[out.state()]);
        res.unwrap();
        op_dag.visit_gen += 1;
        op_dag
            .lower_tree(op_dag.noted.last().unwrap().unwrap(), op_dag.visit_gen)
            .unwrap();
    })
}
