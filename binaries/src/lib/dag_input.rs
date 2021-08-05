use std::rc::Rc;

use awint::dag_prelude::*;

pub fn dag_input() -> Vec<Rc<awint_dag::mimick::State>> {
    let mut awi0 = inlawi!(11001001);
    let mut awi1 = inlawi!(01100011);
    let x = awi0.const_as_mut();
    let y = awi1.const_as_mut();
    x.xor_assign(y).unwrap();
    y.xor_assign(x).unwrap();
    x.xor_assign(y).unwrap();

    let mut awi2 = inlawi!(-1i4);
    let z = awi2.const_as_mut();
    z.not_assign();
    z.not_assign();
    let mut v = Vec::new();
    v.push(<Bits as awint_dag::mimick::Lineage>::state(x));
    v.push(<Bits as awint_dag::mimick::Lineage>::state(z));
    v
}
