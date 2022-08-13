use std::{cmp::min, num::NonZeroUsize};

use awint::prelude as num;
use awint_dag::{
    lowering::Dag, prelude as dag, state::STATE_ARENA, EvalError, Lineage, Op, StateEpoch,
};
use awint_internals::BITS;
use rand_xoshiro::{
    rand_core::{RngCore, SeedableRng},
    Xoshiro128StarStar,
};
use triple_arena::{ptr_struct, Arena};

// miri is just here to check that the unsized deref hacks are working
const N: (u32, u32) = if cfg!(miri) {
    (4, 1)
} else if cfg!(debug_assertions) {
    (32, 100)
} else {
    (32, 1000)
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

    pub fn clear(&mut self) {
        self.a.clear();
        self.v.clear();
        for _ in 0..65 {
            self.v.push(vec![]);
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
        self.a[inx].dag.clone()
    }

    pub fn get_mut_num(&mut self, inx: P0) -> &mut num::ExtAwi {
        &mut self.a[inx].num
    }

    pub fn get_mut_dag(&mut self, inx: P0) -> &mut dag::ExtAwi {
        &mut self.a[inx].dag
    }

    /// Makes sure that plain evaluation works
    pub fn eval_and_verify_equal(&mut self) -> Result<(), EvalError> {
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
            for node in dag.dag.vals() {
                if !matches!(
                    node.op,
                    Op::Opaque(_)
                        | Op::Literal(_)
                        | Op::Copy(_)
                        | Op::StaticGet(_, _)
                        | Op::StaticSet(_, _)
                        | Op::StaticLut(_, _)
                ) {
                    panic!("did not lower all the way: {:?}", node);
                }
            }
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

fn num_dag_duo(rng: &mut Xoshiro128StarStar, m: &mut Mem) {
    let next_op = rng.next_u32() % 26;
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
                    m.get_mut_num(lhs).or_assign(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).or_assign(&rhs_b).unwrap();
                }
                1 => {
                    m.get_mut_num(lhs).and_assign(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).and_assign(&rhs_b).unwrap();
                }
                2 => {
                    m.get_mut_num(lhs).xor_assign(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).xor_assign(&rhs_b).unwrap();
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
        // FieldFrom
        14 => {
            let (w0, lhs) = m.next1_5();
            let (w1, rhs) = m.next1_5();
            let min_w = min(w0, w1);
            let width = m.next_usize(min_w + 1);
            let from = m.next_usize(1 + w1 - m.get_num(width).to_usize());
            let rhs_a = m.get_num(rhs);
            let width_a = m.get_num(width);
            let from_a = m.get_num(from);
            m.get_mut_num(lhs)
                .field_from(&rhs_a, from_a.to_usize(), width_a.to_usize())
                .unwrap();
            let rhs_b = m.get_dag(rhs);
            let width_b = m.get_dag(width);
            let from_b = m.get_dag(from);
            m.get_mut_dag(lhs)
                .field_from(&rhs_b, from_b.to_usize(), width_b.to_usize())
                .unwrap();
        }
        // Shl, Lshr, Ashr, Rotl, Rotr
        15 => {
            let (w, x) = m.next1_5();
            let s = m.next_usize(w);
            let s_a = m.get_num(s);
            let s_b = m.get_dag(s);
            match rng.next_u32() % 5 {
                0 => {
                    m.get_mut_num(x).shl_assign(s_a.to_usize()).unwrap();
                    m.get_mut_dag(x).shl_assign(s_b.to_usize()).unwrap();
                }
                1 => {
                    m.get_mut_num(x).lshr_assign(s_a.to_usize()).unwrap();
                    m.get_mut_dag(x).lshr_assign(s_b.to_usize()).unwrap();
                }
                2 => {
                    m.get_mut_num(x).ashr_assign(s_a.to_usize()).unwrap();
                    m.get_mut_dag(x).ashr_assign(s_b.to_usize()).unwrap();
                }
                3 => {
                    m.get_mut_num(x).rotl_assign(s_a.to_usize()).unwrap();
                    m.get_mut_dag(x).rotl_assign(s_b.to_usize()).unwrap();
                }
                4 => {
                    m.get_mut_num(x).rotr_assign(s_a.to_usize()).unwrap();
                    m.get_mut_dag(x).rotr_assign(s_b.to_usize()).unwrap();
                }
                _ => unreachable!(),
            }
        }
        // FieldTo
        16 => {
            let (w0, lhs) = m.next1_5();
            let (w1, rhs) = m.next1_5();
            let min_w = min(w0, w1);
            let width = m.next_usize(min_w + 1);
            let to = m.next_usize(1 + w0 - m.get_num(width).to_usize());
            let to_a = m.get_num(to);
            let rhs_a = m.get_num(rhs);
            let width_a = m.get_num(width);
            m.get_mut_num(lhs)
                .field_to(to_a.to_usize(), &rhs_a, width_a.to_usize())
                .unwrap();
            let to_b = m.get_dag(to);
            let rhs_b = m.get_dag(rhs);
            let width_b = m.get_dag(width);
            m.get_mut_dag(lhs)
                .field_to(to_b.to_usize(), &rhs_b, width_b.to_usize())
                .unwrap();
        }
        // Add, Sub, Rsb
        17 => {
            let (w, lhs) = m.next1_5();
            let rhs = m.next(w);
            let rhs_a = m.get_num(rhs);
            let rhs_b = m.get_dag(rhs);
            match rng.next_u32() % 3 {
                0 => {
                    m.get_mut_num(lhs).add_assign(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).add_assign(&rhs_b).unwrap();
                }
                1 => {
                    m.get_mut_num(lhs).sub_assign(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).sub_assign(&rhs_b).unwrap();
                }
                2 => {
                    m.get_mut_num(lhs).rsb_assign(&rhs_a).unwrap();
                    m.get_mut_dag(lhs).rsb_assign(&rhs_b).unwrap();
                }
                _ => unreachable!(),
            }
        }
        // Field
        18 => {
            let (w0, lhs) = m.next1_5();
            let (w1, rhs) = m.next1_5();
            let min_w = min(w0, w1);
            let width = m.next_usize(min_w + 1);
            let to = m.next_usize(1 + w0 - m.get_num(width).to_usize());
            let from = m.next_usize(1 + w1 - m.get_num(width).to_usize());

            let to_a = m.get_num(to);
            let rhs_a = m.get_num(rhs);
            let from_a = m.get_num(from);
            let width_a = m.get_num(width);
            m.get_mut_num(lhs)
                .field(
                    to_a.to_usize(),
                    &rhs_a,
                    from_a.to_usize(),
                    width_a.to_usize(),
                )
                .unwrap();
            let to_b = m.get_dag(to);
            let rhs_b = m.get_dag(rhs);
            let from_b = m.get_dag(from);
            let width_b = m.get_dag(width);
            m.get_mut_dag(lhs)
                .field(
                    to_b.to_usize(),
                    &rhs_b,
                    from_b.to_usize(),
                    width_b.to_usize(),
                )
                .unwrap();
        }
        // Rev
        19 => {
            let x = m.next1_5().1;
            m.get_mut_num(x).rev_assign();
            m.get_mut_dag(x).rev_assign();
        }
        // Eq, Ne, Ult, Ule, Ilt, Ile
        20 => {
            let (w, lhs) = m.next1_5();
            let rhs = m.next(w);
            let lhs_a = m.get_num(lhs);
            let lhs_b = m.get_dag(lhs);
            let rhs_a = m.get_num(rhs);
            let rhs_b = m.get_dag(rhs);
            let out = m.next(1);
            match rng.next_u32() % 6 {
                0 => {
                    m.get_mut_num(out)
                        .bool_assign(lhs_a.const_eq(&rhs_a).unwrap());
                    m.get_mut_dag(out)
                        .bool_assign(lhs_b.const_eq(&rhs_b).unwrap());
                }
                1 => {
                    m.get_mut_num(out)
                        .bool_assign(lhs_a.const_ne(&rhs_a).unwrap());
                    m.get_mut_dag(out)
                        .bool_assign(lhs_b.const_ne(&rhs_b).unwrap());
                }
                2 => {
                    m.get_mut_num(out).bool_assign(lhs_a.ult(&rhs_a).unwrap());
                    m.get_mut_dag(out).bool_assign(lhs_b.ult(&rhs_b).unwrap());
                }
                3 => {
                    m.get_mut_num(out).bool_assign(lhs_a.ule(&rhs_a).unwrap());
                    m.get_mut_dag(out).bool_assign(lhs_b.ule(&rhs_b).unwrap());
                }
                4 => {
                    m.get_mut_num(out).bool_assign(lhs_a.ilt(&rhs_a).unwrap());
                    m.get_mut_dag(out).bool_assign(lhs_b.ilt(&rhs_b).unwrap());
                }
                5 => {
                    m.get_mut_num(out).bool_assign(lhs_a.ile(&rhs_a).unwrap());
                    m.get_mut_dag(out).bool_assign(lhs_b.ile(&rhs_b).unwrap());
                }
                _ => unreachable!(),
            }
        }
        // IsZero, IsUmax, IsImax, IsImin, IsUone
        21 => {
            let x = m.next1_5().1;
            let x_a = m.get_num(x);
            let x_b = m.get_dag(x);
            let out = m.next(1);
            match rng.next_u32() % 5 {
                0 => {
                    m.get_mut_num(out).bool_assign(x_a.is_zero());
                    m.get_mut_dag(out).bool_assign(x_b.is_zero());
                }
                1 => {
                    m.get_mut_num(out).bool_assign(x_a.is_umax());
                    m.get_mut_dag(out).bool_assign(x_b.is_umax());
                }
                2 => {
                    m.get_mut_num(out).bool_assign(x_a.is_imax());
                    m.get_mut_dag(out).bool_assign(x_b.is_imax());
                }
                3 => {
                    m.get_mut_num(out).bool_assign(x_a.is_imin());
                    m.get_mut_dag(out).bool_assign(x_b.is_imin());
                }
                4 => {
                    m.get_mut_num(out).bool_assign(x_a.is_uone());
                    m.get_mut_dag(out).bool_assign(x_b.is_uone());
                }
                _ => unreachable!(),
            }
        }
        // CountOnes, Lz, Tz, Sig
        22 => {
            let x = m.next1_5().1;
            let x_a = m.get_num(x);
            let x_b = m.get_dag(x);
            let out = m.next_usize(usize::MAX);
            match rng.next_u32() % 4 {
                0 => {
                    m.get_mut_num(out).usize_assign(x_a.count_ones());
                    m.get_mut_dag(out).usize_assign(x_b.count_ones());
                }
                1 => {
                    m.get_mut_num(out).usize_assign(x_a.lz());
                    m.get_mut_dag(out).usize_assign(x_b.lz());
                }
                2 => {
                    m.get_mut_num(out).usize_assign(x_a.tz());
                    m.get_mut_dag(out).usize_assign(x_b.tz());
                }
                3 => {
                    m.get_mut_num(out).usize_assign(x_a.sig());
                    m.get_mut_dag(out).usize_assign(x_b.sig());
                }
                _ => unreachable!(),
            }
        }
        // LutSet
        23 => {
            let (entry_w, entry) = m.next1_5();
            let (inx_w, inx) = m.next1_5();
            let table_w = entry_w * (1 << inx_w);
            let table = m.next(table_w);
            let entry_a = m.get_num(entry);
            let inx_a = m.get_num(inx);
            m.get_mut_num(table).lut_set(&entry_a, &inx_a).unwrap();
            let entry_b = m.get_dag(entry);
            let inx_b = m.get_dag(inx);
            m.get_mut_dag(table).lut_set(&entry_b, &inx_b).unwrap();
        }
        // Resize
        24 => {
            let lhs = m.next1_5().1;
            let rhs = m.next1_5().1;
            let b = m.next(1);
            let rhs_a = m.get_num(rhs);
            let b_a = m.get_num(b);
            m.get_mut_num(lhs).resize_assign(&rhs_a, b_a.to_bool());
            let rhs_b = m.get_dag(rhs);
            let b_b = m.get_dag(b);
            m.get_mut_dag(lhs).resize_assign(&rhs_b, b_b.to_bool());
        }
        // ZeroResizeOverflow
        25 => {
            let lhs = m.next1_5().1;
            let rhs = m.next1_5().1;
            let b = m.next(1);
            let rhs_a = m.get_num(rhs);
            let b_a = m.get_num(b);
            m.get_mut_num(lhs).resize_assign(&rhs_a, b_a.to_bool());
            let rhs_b = m.get_dag(rhs);
            let b_b = m.get_dag(b);
            m.get_mut_dag(lhs).resize_assign(&rhs_b, b_b.to_bool());
        }
        _ => unreachable!(),
    }
}

#[test]
fn dag_fuzzing() {
    let mut rng = Xoshiro128StarStar::seed_from_u64(0);
    let mut m = Mem::new();

    for _ in 0..N.1 {
        let epoch = StateEpoch::new();
        m.clear();
        for _ in 0..N.0 {
            num_dag_duo(&mut rng, &mut m)
        }
        m.eval_and_verify_equal().unwrap();
        let res = m.lower_and_verify_equal();
        res.unwrap();
        drop(epoch);
        assert!(STATE_ARENA.with(|f| f.borrow().is_empty()));
    }
    /*let mut leaves = vec![];
    for val in m.a.vals() {
        leaves.push(val.dag.state());
    }
    let (mut dag, _) = Dag::new(&leaves, &leaves);
    dag.render_to_svg_file(std::path::PathBuf::from("rendered.svg"))
        .unwrap();*/
}

#[test]
fn state_epochs() {
    use awint_dag::primitive::u8;
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
