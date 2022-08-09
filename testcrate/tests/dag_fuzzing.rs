use std::{cmp::min, num::NonZeroUsize};

use awint::prelude as num;
use awint_dag::{
    common::{EvalError, Lineage, Op},
    lowering::Dag,
    prelude as dag,
};
use awint_internals::BITS;
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};
use triple_arena::{ptr_struct, Arena};

// miri is just here to check that the unsized deref hacks are working
const N: u32 = if cfg!(miri) {
    32
} else if cfg!(debug_assertions) {
    1000
} else {
    10000
};

ptr_struct!(P0);

#[derive(Debug, Clone)]
struct Pair {
    num: num::ExtAwi,
    dag: dag::ExtAwi,
}

impl Pair {
    pub fn new(lit: num::ExtAwi) -> Self {
        Self {
            num: lit.clone(),
            dag: lit.const_as_ref().into(),
        }
    }
}

#[derive(Debug)]
struct Mem {
    a: Arena<P0, Pair>,
    // The outer Vec has 65 Vecs for all the supported bitwidths (there is a dummy 0 bitwidth Vec
    // and one for each of 1..=64), the inner Vecs are unsorted and used for random querying
    v: Vec<Vec<P0>>,
    rng: Xoshiro128StarStar,
}

impl Mem {
    pub fn new() -> Self {
        let mut v = vec![];
        for _ in 0..65 {
            v.push(vec![]);
        }
        Self {
            a: Arena::<P0, Pair>::new(),
            v,
            rng: Xoshiro128StarStar::seed_from_u64(0),
        }
    }

    /// Randomly creates a new pair or gets an existing one under the `cap`
    pub fn next_capped(&mut self, w: usize, cap: usize) -> P0 {
        let try_query = (self.rng.next_u32() % 4) != 0;
        if try_query && (!self.v[w].is_empty()) {
            let p = self.v[w][(self.rng.next_u32() as usize) % self.v[w].len()];
            if self.get_num(p).to_usize() < cap {
                return p
            }
        }
        let mut lit = num::ExtAwi::zero(NonZeroUsize::new(w).unwrap());
        lit.rand_assign_using(&mut self.rng).unwrap();
        let tmp = lit.to_usize() % cap;
        lit.usize_assign(tmp);
        let p = self.a.insert(Pair::new(lit));
        self.v[w].push(p);
        p
    }

    /// Randomly creates a new pair or gets an existing one
    pub fn next(&mut self, w: usize) -> P0 {
        let try_query = (self.rng.next_u32() % 4) != 0;
        if try_query && (!self.v[w].is_empty()) {
            self.v[w][(self.rng.next_u32() as usize) % self.v[w].len()]
        } else {
            let mut lit = num::ExtAwi::zero(NonZeroUsize::new(w).unwrap());
            lit.rand_assign_using(&mut self.rng).unwrap();
            let p = self.a.insert(Pair::new(lit));
            self.v[w].push(p);
            p
        }
    }

    /// Calls `next` with a random integer in 1..5, returning a tuple of the
    /// width chosen and the Ptr to what `next` returned.
    pub fn next1_5(&mut self) -> (usize, P0) {
        let w = ((self.rng.next_u32() as usize) % 4) + 1;
        (w, self.next(w))
    }

    pub fn next_usize(&mut self, cap: usize) -> P0 {
        self.next_capped(BITS, cap)
    }

    // just use cloning for the immutable indexing, because dealing with the guards
    // of mixed internal mutability is too much. We can't get the signature of
    // `Index` to work in any case.

    pub fn get_num(&self, inx: P0) -> num::ExtAwi {
        self.a[inx].num.clone()
    }

    pub fn get_dag(&self, inx: P0) -> dag::ExtAwi {
        self.a[inx].dag.unstable_clone_identical()
    }

    pub fn get_mut_num(&mut self, inx: P0) -> &mut num::ExtAwi {
        &mut self.a[inx].num
    }

    pub fn get_mut_dag(&mut self, inx: P0) -> &mut dag::ExtAwi {
        &mut self.a[inx].dag
    }

    /// Makes sure that plain evaluation works
    pub fn eval_and_verify_equal(&mut self) -> Result<(), EvalError> {
        // TODO lower leaves primarily in this function and below
        for pair in self.a.vals() {
            let (mut dag, res) = Dag::new(&[pair.dag.state()], &[pair.dag.state()]);
            res?;
            let leaf = dag.noted[0].unwrap();
            dag.visit_gen += 1;
            dag.eval_tree(leaf, dag.visit_gen)?;
            if let Op::Literal(ref lit) = dag[leaf].op {
                if pair.num != *lit {
                    return Err(EvalError::OtherStr("real and mimick mismatch"))
                }
            }
        }
        Ok(())
    }

    /// Makes sure that lowering works
    pub fn lower_and_verify_equal(&mut self) -> Result<(), EvalError> {
        for pair in self.a.vals() {
            let (mut dag, res) = Dag::new(&[pair.dag.state()], &[pair.dag.state()]);
            res?;
            // if all constants are known, the lowering will simply become an evaluation. We
            // convert half of the literals to opaques at random, lower the dag, and finally
            // convert back and evaluate to check that the lowering did not break the DAG
            // function.
            let mut literals = vec![];
            for (p, node) in &mut dag.dag {
                if node.op.is_literal() && ((self.rng.next_u32() & 1) == 0) {
                    if let Op::Literal(lit) = node.op.take() {
                        literals.push((p, lit));
                        node.op = Op::Opaque(vec![]);
                    } else {
                        unreachable!()
                    }
                }
            }
            let leaf = dag.noted[0].unwrap();
            dag.visit_gen += 1;
            let res = dag.lower_tree(leaf, dag.visit_gen);
            res.unwrap();
            //dag.render_to_svg_file(std::path::PathBuf::from("rendered.svg"))
            //    .unwrap();
            for (p, lit) in literals {
                if let Some(op) = dag.dag.get_mut(p) {
                    // we are not respecting gen counters in release mode so we need this check
                    if op.op.is_opaque() {
                        op.op = Op::Literal(lit);
                    }
                } // else the literal was culled
            }
            dag.visit_gen += 1;
            dag.eval_tree(leaf, dag.visit_gen)?;
            if let Op::Literal(ref lit) = dag[leaf].op {
                if pair.num != *lit {
                    return Err(EvalError::OtherStr("real and mimick mismatch"))
                }
            } else {
                panic!("did not eval to literal")
            }
        }
        Ok(())
    }
}

#[test]
fn dag_fuzzing() {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut m = Mem::new();

    for _ in 0..N {
        let next_op = rng.next_u32() % 14;
        match next_op {
            // Lut, StaticLut
            0 => {
                let (out_w, out) = m.next1_5();
                let (inx_w, inx) = m.next1_5();
                let lut = m.next(out_w * (1 << inx_w));
                let lut_a = m.get_num(lut);
                let inx_a = m.get_num(inx);
                m.get_mut_num(out).lut(&lut_a, &inx_a).unwrap();
                let lut_b = m.get_dag(lut);
                let inx_b = m.get_dag(inx);
                m.get_mut_dag(out).lut(&lut_b, &inx_b).unwrap();
            }
            // Get, StaticGet
            1 => {
                let (bits_w, bits) = m.next1_5();
                let inx = m.next_usize(bits_w);
                let out = m.next(1);
                let bits_a = m.get_num(bits);
                let inx_a = m.get_num(inx);
                m.get_mut_num(out)
                    .bool_assign(bits_a.get(inx_a.to_usize()).unwrap());
                let bits_b = m.get_dag(bits);
                let inx_b = m.get_dag(inx);
                m.get_mut_dag(out)
                    .bool_assign(bits_b.get(inx_b.to_usize()).unwrap());
            }
            // Set, StaticSet
            2 => {
                let (bits_w, bits) = m.next1_5();
                let inx = m.next_usize(bits_w);
                let bit = m.next(1);
                let inx_a = m.get_num(inx);
                let bit_a = m.get_num(bit);
                m.get_mut_num(bits)
                    .set(inx_a.to_usize(), bit_a.to_bool())
                    .unwrap();
                let inx_b = m.get_dag(inx);
                let bit_b = m.get_dag(bit);
                m.get_mut_dag(bits)
                    .set(inx_b.to_usize(), bit_b.to_bool())
                    .unwrap();
            }
            // FieldBit
            3 => {
                let (lhs_w, lhs) = m.next1_5();
                let to = m.next_usize(lhs_w);
                let (rhs_w, rhs) = m.next1_5();
                let from = m.next_usize(rhs_w);
                let to_a = m.get_num(to);
                let rhs_a = m.get_num(rhs);
                let from_a = m.get_num(from);
                m.get_mut_num(lhs)
                    .field_bit(to_a.to_usize(), &rhs_a, from_a.to_usize())
                    .unwrap();
                let to_b = m.get_dag(to);
                let rhs_b = m.get_dag(rhs);
                let from_b = m.get_dag(from);
                m.get_mut_dag(lhs)
                    .field_bit(to_b.to_usize(), &rhs_b, from_b.to_usize())
                    .unwrap();
            }
            // ZeroResize
            4 => {
                let lhs = m.next1_5().1;
                let rhs = m.next1_5().1;
                let rhs_a = m.get_num(rhs);
                m.get_mut_num(lhs).zero_resize_assign(&rhs_a);
                let rhs_b = m.get_dag(rhs);
                m.get_mut_dag(lhs).zero_resize_assign(&rhs_b);
            }
            // SignResize
            5 => {
                let lhs = m.next1_5().1;
                let rhs = m.next1_5().1;
                let rhs_a = m.get_num(rhs);
                m.get_mut_num(lhs).sign_resize_assign(&rhs_a);
                let rhs_b = m.get_dag(rhs);
                m.get_mut_dag(lhs).sign_resize_assign(&rhs_b);
            }
            // Not
            6 => {
                let x = m.next1_5().1;
                m.get_mut_num(x).not_assign();
                m.get_mut_dag(x).not_assign();
            }
            // Or, And, Xor
            7 => {
                let (lhs_w, lhs) = m.next1_5();
                let rhs = m.next(lhs_w);
                let rhs_a = m.get_num(rhs);
                let rhs_b = m.get_dag(rhs);
                match rng.next_u32() % 3 {
                    0 => {
                        m.get_mut_num(lhs).or_assign(&rhs_a);
                        m.get_mut_dag(lhs).or_assign(&rhs_b);
                    }
                    1 => {
                        m.get_mut_num(lhs).and_assign(&rhs_a);
                        m.get_mut_dag(lhs).and_assign(&rhs_b);
                    }
                    2 => {
                        m.get_mut_num(lhs).xor_assign(&rhs_a);
                        m.get_mut_dag(lhs).xor_assign(&rhs_b);
                    }
                    _ => unreachable!(),
                }
            }
            // Inc, IncCout, Dec, DecCout
            8 => {
                let x = m.next1_5().1;
                let cin = m.next(1);
                let cout = m.next(1);
                let cin_a = m.get_num(cin);
                let cin_b = m.get_dag(cin);
                let out_a;
                let out_b;
                if (rng.next_u32() & 1) == 0 {
                    out_a = m.get_mut_num(x).inc_assign(cin_a.to_bool());
                    out_b = m.get_mut_dag(x).inc_assign(cin_b.to_bool());
                } else {
                    out_a = m.get_mut_num(x).dec_assign(cin_a.to_bool());
                    out_b = m.get_mut_dag(x).dec_assign(cin_b.to_bool());
                }
                m.get_mut_num(cout).bool_assign(out_a);
                m.get_mut_dag(cout).bool_assign(out_b);
            }
            // CinSum, UnsignedOverflow, SignedOverflow
            9 => {
                let cin = m.next(1);
                let (lhs_w, lhs) = m.next1_5();
                let rhs = m.next(lhs_w);
                let out = m.next(lhs_w);
                let unsigned = m.next(1);
                let signed = m.next(1);

                let cin_a = m.get_num(cin);
                let lhs_a = m.get_num(lhs);
                let rhs_a = m.get_num(rhs);
                let overflow = m
                    .get_mut_num(out)
                    .cin_sum_assign(cin_a.to_bool(), &lhs_a, &rhs_a)
                    .unwrap();
                m.get_mut_num(unsigned).bool_assign(overflow.0);
                m.get_mut_num(signed).bool_assign(overflow.1);

                let cin_b = m.get_dag(cin);
                let lhs_b = m.get_dag(lhs);
                let rhs_b = m.get_dag(rhs);
                let overflow = m
                    .get_mut_dag(out)
                    .cin_sum_assign(cin_b.to_bool(), &lhs_b, &rhs_b)
                    .unwrap();
                m.get_mut_dag(unsigned).bool_assign(overflow.0);
                m.get_mut_dag(signed).bool_assign(overflow.1);
            }
            // Lsb, Msb
            10 => {
                let x = m.next1_5().1;
                let out = m.next(1);
                if (rng.next_u32() & 1) == 0 {
                    let a = m.get_num(x).lsb();
                    m.get_mut_num(out).bool_assign(a);
                    let b = m.get_dag(x).lsb();
                    m.get_mut_dag(out).bool_assign(b);
                } else {
                    let a = m.get_num(x).msb();
                    m.get_mut_num(out).bool_assign(a);
                    let b = m.get_dag(x).msb();
                    m.get_mut_dag(out).bool_assign(b);
                }
            }
            // Neg, Abs
            11 => {
                let x = m.next1_5().1;
                if (rng.next_u32() & 1) == 0 {
                    let neg = m.next(1);
                    let a = m.get_num(neg).to_bool();
                    m.get_mut_num(x).neg_assign(a);
                    let b = m.get_dag(neg).to_bool();
                    m.get_mut_dag(x).neg_assign(b);
                } else {
                    m.get_mut_num(x).abs_assign();
                    m.get_mut_dag(x).abs_assign();
                }
            }
            // Funnel
            12 => {
                let w = 1 << (((m.rng.next_u32() as usize) % 2) + 1);
                let lhs = m.next(w);
                let rhs = m.next(w * 2);
                let s = m.next(w.trailing_zeros() as usize);
                let a = m.get_num(rhs);
                let a_s = m.get_num(s);
                m.get_mut_num(lhs).funnel(&a, &a_s).unwrap();
                let b = m.get_dag(rhs);
                let b_s = m.get_dag(s);
                m.get_mut_dag(lhs).funnel(&b, &b_s).unwrap();
            }
            // FieldWidth
            13 => {
                let (w0, lhs) = m.next1_5();
                let (w1, rhs) = m.next1_5();
                let min_w = min(w0, w1);
                let width = m.next_usize(min_w + 1);
                let rhs_a = m.get_num(rhs);
                let width_a = m.get_num(width);
                m.get_mut_num(lhs)
                    .field_width(&rhs_a, width_a.to_usize())
                    .unwrap();
                let rhs_b = m.get_dag(rhs);
                let width_b = m.get_dag(width);
                m.get_mut_dag(lhs)
                    .field_width(&rhs_b, width_b.to_usize())
                    .unwrap();
            }
            _ => unreachable!(),
        }
    }
    m.eval_and_verify_equal().unwrap();
    let res = m.lower_and_verify_equal();
    /*let mut leaves = vec![];
    for val in m.a.vals() {
        leaves.push(val.dag.state());
    }
    let (mut dag, _) = Dag::new(&leaves, &leaves);
    dag.render_to_svg_file(std::path::PathBuf::from("rendered.svg"))
        .unwrap();*/
    res.unwrap();
}
