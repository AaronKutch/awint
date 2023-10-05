#![feature(test)]

extern crate test;
use awint::{
    awi,
    awint_dag::{basic_state_epoch::StateEpoch, Lineage, OpDag},
    awint_macro_internals::triple_arena::ptr_struct,
    dag::*,
};
use test::Bencher;

ptr_struct!(P0; P1; P2);

#[bench]
fn lower_funnel(bencher: &mut Bencher) {
    bencher.iter(|| {
        let epoch0 = StateEpoch::new();
        let mut out = inlawi!(0u32);
        let rhs = inlawi!(opaque: ..64);
        let s = inlawi!(opaque: ..5);
        out.funnel_(&rhs, &s).unwrap();

        let (mut op_dag, res) = OpDag::from_epoch(&epoch0);
        res.unwrap();
        op_dag.note_pstate(&epoch0, out.state()).unwrap();
        op_dag.lower_all().unwrap();
        awi::assert_eq!(op_dag.a.len(), 7044);
    })
}
