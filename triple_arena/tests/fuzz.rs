use std::collections::VecDeque;

use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};
use triple_arena::prelude::*;

#[test]
fn fuzz() {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);

    // unique id for checking that the correct elements are returned
    let mut counter = 0u64;
    let mut new_id = || {
        counter += 1;
        counter
    };

    let mut m: Vec<Arena<u64>> = Vec::new();
    // keeps track of elements in `m` for checking correctness. The tuple is of
    // (unique id, `m` arena index, pointer).
    let mut queue: VecDeque<(u64, usize, TriPtr)> = VecDeque::new();
    // arenas of various sizes
    let sizes = 4;
    for i in 0..sizes {
        let mut ta = Arena::new();
        for _ in 0..(1 << i) {
            let id = new_id();
            queue.push_back((id, i, ta.insert(id)));
        }
        m.push(ta);
    }
    let mut tot_len = 0;
    for i in 0..sizes {
        tot_len += m[i].len();
    }
    assert_eq!(tot_len, 15);

    let mut saturations = 0;
    for _ in 0..100_000 {
        match rng.next_u32() % 8 {
            // make removals as common as insertions
            0 | 1 => {
                if queue.len() == 0 {
                    // no elements in `m`, lets test `insert`
                    saturations += 1;
                    for m_i in 0..sizes {
                        assert_eq!(m[m_i].len(), 0);
                        for _ in 0..(1 << m_i) {
                            let id = new_id();
                            queue.push_back((id, m_i, m[m_i].insert(id)));
                        }
                        assert_eq!(m[m_i].len(), 1 << m_i);
                        assert_eq!(m[m_i].capacity(), 1 << m_i);
                    }
                } else {
                    let queue_i = (rng.next_u32() as usize) % queue.len();
                    let (id, m_i, ptr) = queue.swap_remove_back(queue_i).unwrap();
                    assert_eq!(&id, m[m_i].get(ptr).unwrap());
                    assert_eq!(&id, m[m_i].get_mut(ptr).unwrap());
                    assert_eq!(id, m[m_i].remove(ptr).unwrap());
                }
            }
            2 => {
                // test `try_insert`
                // treat `m` as one contiguous arena and figure out which to insert into
                let sig_bits =
                    (32 - (rng.next_u32() % (1 << (sizes - 1))).leading_zeros()) as usize;
                let m_i = (sizes - 1) - sig_bits;
                if m[m_i].len() < (1 << m_i) {
                    let id = new_id();
                    queue.push_back((id, m_i, m[m_i].try_insert(id).unwrap()));
                } else {
                    assert_eq!(m[m_i].try_insert(!0), Err(!0));
                }
            }
            3 => {
                // test `try_insert_with`
                let sig_bits =
                    (32 - (rng.next_u32() % (1 << (sizes - 1))).leading_zeros()) as usize;
                let m_i = (sizes - 1) - sig_bits;
                if m[m_i].len() < (1 << m_i) {
                    let id = new_id();
                    queue.push_back((
                        id,
                        m_i,
                        m[m_i].try_insert_with(|| id).unwrap_or_else(|_| panic!()),
                    ));
                } else {
                    assert!(m[m_i].try_insert_with(|| !0).is_err());
                }
            }
            4 => {
                // test `replace_update_gen`
                if queue.len() == 0 {
                    // nothing to replace
                    continue
                }
                let queue_i = (rng.next_u32() as usize) % queue.len();
                let (id, m_i, old_ptr) = queue.swap_remove_back(queue_i).unwrap();
                let nid = new_id();
                if let Ok((new_ptr, old)) = m[m_i].replace_update_gen(old_ptr, nid) {
                    assert_eq!(id, old);
                    queue.push_back((nid, m_i, new_ptr));
                } else {
                    panic!();
                }
            }
            5 => {
                // test `replace_keep_gen`
                if queue.len() == 0 {
                    // nothing to replace
                    continue
                }
                let queue_i = (rng.next_u32() as usize) % queue.len();
                let (id, m_i, ptr) = queue.swap_remove_back(queue_i).unwrap();
                let nid = new_id();
                if let Ok(old) = m[m_i].replace_keep_gen(ptr, nid) {
                    assert_eq!(id, old);
                    queue.push_back((nid, m_i, ptr));
                } else {
                    panic!();
                }
            }
            6 => {
                // test miscellanious functions
                if queue.len() == 0 {
                    // the removal section tests a bunch of stuff
                    continue
                }
                let queue_i = (rng.next_u32() as usize) % queue.len();
                let (id, m_i, ptr) = queue[queue_i];
                assert_eq!(id, *m[m_i].get(ptr).unwrap());
                assert_eq!(id, *m[m_i].get_mut(ptr).unwrap());
                let new_ptr = m[m_i].invalidate(ptr).unwrap();
                queue[queue_i].2 = new_ptr;
            }
            7 => {
                // test failures
                if queue.len() == 0 {
                    continue
                }
                let i0 = (rng.next_u32() as usize) % queue.len();
                let i1 = (rng.next_u32() as usize) % queue.len();
                let (_, m_i0, ptr0) = queue[i0];
                let (_, m_i1, ptr1) = queue[i1];
                if m_i0 != m_i1 {
                    // differing arenas
                    assert!(m[m_i0].get(ptr1).is_none());
                    assert!(m[m_i0].get_mut(ptr1).is_none());
                    assert!(m[m_i0].invalidate(ptr1).is_none());
                    assert!(m[m_i0].replace_keep_gen(ptr1, 0).is_err());
                    assert!(m[m_i0].replace_update_gen(ptr1, 0).is_err());
                    assert!(m[m_i0].remove(ptr1).is_none());
                }
                // differing gens
                let new_ptr = m[m_i0].invalidate(ptr0).unwrap();
                queue[i0].2 = new_ptr;
                assert!(m[m_i0].get(ptr0).is_none());
                assert!(m[m_i0].get_mut(ptr0).is_none());
                assert!(m[m_i0].invalidate(ptr0).is_none());
                assert!(m[m_i0].replace_keep_gen(ptr0, 0).is_err());
                assert!(m[m_i0].replace_update_gen(ptr0, 0).is_err());
                assert!(m[m_i0].remove(ptr0).is_none());
            }
            _ => panic!(),
        }
    }
    // make sure something is not completely broken in some way by checking if the
    // different extremes have been reached many times
    if saturations < 500 {
        panic!();
    }
}
