use awint::{
    awi,
    awint_dag::{state::STATE_ARENA, Lineage, OpDag, StateEpoch},
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
        assert!(state.get_state().is_some());
        state
    };
    assert!(state.get_state().is_none());
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
    let mut noted = vec![];
    let assertions_start = noted.len();
    noted.extend(epoch0.assertions().states());
    let (mut graph, res) = OpDag::new(&noted, &noted);
    res.unwrap();
    graph.eval_all_noted().unwrap();
    for i in assertions_start..noted.len() {
        use awi::{assert_eq, *};
        assert_eq!(graph.lit(graph.noted[i].unwrap()), inlawi!(1).as_ref());
    }
}
