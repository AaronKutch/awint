#![allow(renamed_and_removed_lints)]
#![allow(clippy::thread_local_initializer_can_be_made_const)]

//! for `awint_dag` mimicking only testing. There are a few cases that are
//! really only tested well in `starlight`

use core::fmt;
use std::{
    borrow::Borrow,
    cell::RefCell,
    fmt::Write,
    num::{NonZeroU64, NonZeroUsize},
    ops::Deref,
    thread::panicking,
};

use awint::{
    awi,
    awint_dag::{
        dag,
        epoch::{EpochCallback, EpochKey},
        triple_arena::Arena,
        triple_arena_render::{self, DebugNode, DebugNodeTrait},
        EAwi, EvalResult, Lineage, Location, Op, PState,
    },
    bw, Awi,
};

/// Represents a single state that `mimick::Bits` is in at one point in time.
/// The operands point to other `State`s. `Bits` and `*Awi` use `Ptr`s to
/// `States` in a thread local arena, so that they can change their
/// state without borrowing issues or mutating `States` (which could be used as
/// operands by other `States`).
#[derive(Debug, Clone)]
pub struct State {
    /// Bitwidth
    pub nzbw: NonZeroUsize,
    /// Operation
    pub op: Op<PState>,
    /// Location where this state is derived from
    pub location: Option<Location>,
    /// Errors
    pub err: Option<String>,
    /// Used in algorithms
    pub visit: NonZeroU64,
}

impl DebugNodeTrait<PState> for State {
    fn debug_node(p_this: PState, this: &Self) -> DebugNode<PState> {
        let names = this.op.operand_names();
        let mut res = DebugNode {
            sources: this
                .op
                .operands()
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    (
                        *p,
                        if names.len() > 1 {
                            names[i].to_owned()
                        } else {
                            String::new()
                        },
                    )
                })
                .collect(),
            center: match this.op {
                Op::Opaque(..) => vec![format!("{} {}", this.op.operation_name(), this.nzbw)],
                Op::Literal(ref awi) => vec![format!("{awi}")],
                Op::StaticLut(_, ref awi) => vec![format!("lut {awi}")],
                _ => vec![this.op.operation_name().to_owned()],
            },
            sinks: vec![],
        };
        if let Some(ref err) = this.err {
            res.center.push(format!("ERROR: {err:?}"));
        }
        res.center.push(format!("{}", this.nzbw));
        if let Some(location) = this.location {
            res.center.push(format!("{location:?}"));
        }
        res.center.push(format!("{p_this:?}"));
        res
    }
}

struct EpochData {
    key: EpochKey,
    assertions: Vec<PState>,
    states: Arena<PState, State>,
    visit_gen: NonZeroU64,
}

// See `starlight` for more complicated implementations
thread_local!(
    static EPOCH_DATA: RefCell<Vec<EpochData>> = RefCell::new(vec![]);
);

pub fn _test_callback() -> EpochCallback {
    fn new_pstate(nzbw: NonZeroUsize, op: Op<PState>, location: Option<Location>) -> PState {
        EPOCH_DATA.with(|stack| {
            let mut stack = stack.borrow_mut();
            let top = stack.last_mut().unwrap();
            let visit = top.visit_gen;
            top.states.insert(State {
                nzbw,
                op,
                location,
                err: None,
                visit,
            })
        })
    }
    fn register_assertion_bit(bit: dag::bool, location: Location) {
        EPOCH_DATA.with(|stack| {
            let mut stack = stack.borrow_mut();
            let top = stack.last_mut().unwrap();
            let visit = top.visit_gen;
            let p_state = top.states.insert(State {
                nzbw: bw(1),
                op: Op::Assert([bit.state()]),
                location: Some(location),
                err: None,
                visit,
            });
            top.assertions.push(p_state);
        });
    }
    fn get_nzbw(p_state: PState) -> NonZeroUsize {
        EPOCH_DATA.with(|stack| {
            let stack = stack.borrow();
            let top = stack.last().unwrap();
            top.states.get(p_state).unwrap().nzbw
        })
    }
    fn get_op(p_state: PState) -> Op<PState> {
        EPOCH_DATA.with(|stack| {
            let stack = stack.borrow();
            let top = stack.last().unwrap();
            top.states.get(p_state).unwrap().op.clone()
        })
    }
    EpochCallback {
        new_pstate,
        register_assertion_bit,
        get_nzbw,
        get_op,
    }
}

#[derive(Debug)]
pub struct Epoch {
    key: EpochKey,
}

impl Drop for Epoch {
    fn drop(&mut self) {
        // prevent invoking recursive panics and a buffer overrun
        if !panicking() {
            EPOCH_DATA.with(|top| {
                let mut top = top.borrow_mut();
                let last = top.pop().unwrap();
                assert_eq!(
                    last.key, self.key,
                    "`Epoch` was not created and dropped in a stacklike way"
                );
            });
            // unregister callback
            self.key.pop_off_epoch_stack().unwrap();
        }
    }
}

impl Epoch {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let key = _test_callback().push_on_epoch_stack();
        EPOCH_DATA.with(|top| {
            let mut top = top.borrow_mut();
            top.push(EpochData {
                assertions: vec![],
                key,
                visit_gen: NonZeroU64::new(2).unwrap(),
                states: Arena::new(),
            })
        });
        Self { key }
    }

    /// Gets the assertions associated with this epoch (not including assertions
    /// from when sub-epochs are alive or from before the this epoch was
    /// created)
    pub fn assertions(&self) -> Vec<PState> {
        let mut res = vec![];
        EPOCH_DATA.with(|stack| {
            let stack = stack.borrow();
            for (i, layer) in stack.iter().enumerate().rev() {
                if layer.key == self.key {
                    res = layer.assertions.clone();
                    break
                }
                if i == 0 {
                    // shouldn't be reachable even with leaks
                    unreachable!();
                }
            }
        });
        res
    }

    pub fn get_states<F: FnMut(&Arena<PState, State>)>(&self, mut f: F) {
        EPOCH_DATA.with(|stack| {
            let mut stack = stack.borrow_mut();
            let top = stack.last_mut().unwrap();
            assert_eq!(top.key, self.key);
            f(&top.states)
        })
    }

    pub fn assert_assertions(&self) -> Result<(), String> {
        for assertion in &self.assertions() {
            let eval = eval_thread_local_state(*assertion);
            if eval != Ok(Awi::from_bool(true)) {
                return Err(format!("{assertion} {eval:?}"))
            }
        }
        Ok(())
    }

    pub fn _render_to_svg_file(&self, out_file: std::path::PathBuf) {
        self.get_states(|states| {
            let out_file = out_file.clone();
            triple_arena_render::render_to_svg_file(states, false, out_file).unwrap();
        });
    }
}

fn get_thread_local_state_mut<O, F: FnMut(&mut State) -> O>(p_state: PState, mut f: F) -> O {
    EPOCH_DATA.with(|stack| {
        let mut stack = stack.borrow_mut();
        let top = stack.last_mut().unwrap();
        let state = top.states.get_mut(p_state).unwrap();
        f(state)
    })
}

fn eval_thread_local_state(p_state: PState) -> Result<Awi, String> {
    let mut res = None;
    EPOCH_DATA.with(|stack| {
        let mut stack = stack.borrow_mut();
        let top = stack.last_mut().unwrap();
        top.visit_gen = NonZeroU64::new(top.visit_gen.get().checked_add(1).unwrap()).unwrap();
        let visit = top.visit_gen;
        let states = &mut top.states;
        // DFS to evaluate the states down to literals
        let mut path: Vec<(usize, PState, bool)> = vec![(0, p_state, true)];
        loop {
            let (i, p, all_literals) = path[path.len() - 1];
            let ops = states[p].op.operands();
            if ops.is_empty() {
                // reached a root
                path.pop().unwrap();
                if path.is_empty() {
                    break
                }
                path.last_mut().unwrap().0 += 1;
                path.last_mut().unwrap().2 &= states[p].op.is_literal();
            } else if i >= ops.len() {
                // checked all sources
                path.pop().unwrap();
                if all_literals {
                    let self_w = states[p].nzbw;
                    let lit_op: Op<EAwi> =
                        Op::translate(&states[p].op, |lhs: &mut [EAwi], rhs: &[PState]| {
                            for (lhs, rhs) in lhs.iter_mut().zip(rhs.iter()) {
                                if let Op::Literal(ref lit) = states[rhs].op {
                                    *lhs = EAwi::KnownAwi(lit.to_owned());
                                } else {
                                    unreachable!()
                                }
                            }
                        });
                    let eval_res = match lit_op.eval(self_w) {
                        EvalResult::Valid(x) | EvalResult::Pass(x) => {
                            states[p].op = Op::Literal(x);
                            Ok(())
                        }
                        EvalResult::Noop => {
                            let operands = states[p].op.operands();
                            let mut s = String::new();
                            for op in operands {
                                write!(s, "{:?}, ", states[op]).unwrap();
                            }
                            Err(format!(
                                "`EvalResult::Noop` failure on {} {:?} (\n{}\n)",
                                p, states[p].op, s
                            ))
                        }
                        EvalResult::Unevaluatable | EvalResult::PassUnevaluatable => {
                            Err("unevaluatable".to_owned())
                        }
                        EvalResult::AssertionSuccess => {
                            if let Op::Assert([_]) = states[p].op {
                                states[p].op = Op::Literal(Awi::umax(bw(1)));
                                Ok(())
                            } else {
                                unreachable!()
                            }
                        }
                        EvalResult::AssertionFailure => Err(format!(
                            "`EvalResult::AssertionFailure` (\n{:?}\n) on {:?}",
                            p, states[p].op
                        )),
                        EvalResult::Error(e) => {
                            let operands = states[p].op.operands();
                            let mut s = String::new();
                            for op in operands {
                                write!(s, "{:?}, ", states[op]).unwrap();
                            }
                            Err(format!(
                                "`EvalResult::Error` failure (\n{:?}\n) on {} {:?} (\n{}\n)",
                                e, p, states[p].op, s
                            ))
                        }
                    };
                    match eval_res {
                        Ok(()) => {}
                        Err(e) => {
                            states[p].err = Some(e.clone());
                            res = Some(Err(e));
                            return
                        }
                    }
                }
                if path.is_empty() {
                    break
                }
                path.last_mut().unwrap().2 &= all_literals;
            } else {
                let p_next = ops[i];
                if states[p_next].visit >= visit {
                    // peek at node for evaluatableness but do not visit node, this prevents
                    // exponential growth
                    path.last_mut().unwrap().0 += 1;
                    path.last_mut().unwrap().2 &= states[p_next].op.is_literal();
                } else {
                    states[p_next].visit = visit;
                    path.push((0, p_next, true));
                }
            }
        }
        if let Op::Literal(ref lit) = states[p_state].op {
            res = Some(Ok(lit.clone()));
        } else {
            res = Some(Err(format!("`could not eval to a literal {}", p_state)));
        }
    });
    res.unwrap()
}

pub struct LazyAwi {
    opaque: dag::Awi,
    nzbw: NonZeroUsize,
}

impl Lineage for LazyAwi {
    fn state(&self) -> PState {
        self.opaque.state()
    }
}

impl LazyAwi {
    fn internal_as_ref(&self) -> &dag::Bits {
        &self.opaque
    }

    pub fn opaque(w: NonZeroUsize) -> Self {
        Self {
            opaque: dag::Awi::opaque(w),
            nzbw: w,
        }
    }

    /// Retroactively-assigns by `rhs`.
    pub fn retro_(&self, rhs: &awi::Bits) -> Result<(), String> {
        if self.nzbw != rhs.nzbw() {
            return Err("bitwidth mismatch".to_owned());
        }
        let p_lhs = self.state();
        get_thread_local_state_mut(p_lhs, |state| {
            if state.op.is_opaque() {
                state.op = Op::Literal(awi::Awi::from(rhs));
                Ok(())
            } else {
                Err("this testing `LazyAwi` struct cannot be assigned to more than once".to_owned())
            }
        })
    }
}

impl Deref for LazyAwi {
    type Target = dag::Bits;

    fn deref(&self) -> &Self::Target {
        self.internal_as_ref()
    }
}

impl Borrow<dag::Bits> for LazyAwi {
    fn borrow(&self) -> &dag::Bits {
        self
    }
}

impl AsRef<dag::Bits> for LazyAwi {
    fn as_ref(&self) -> &dag::Bits {
        self
    }
}

impl fmt::Debug for LazyAwi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LazyAwi({:?})", self.state())
    }
}
pub struct EvalAwi {
    p_state: PState,
}

impl Lineage for EvalAwi {
    fn state(&self) -> PState {
        self.p_state
    }
}

impl EvalAwi {
    pub fn from_bits(bits: &dag::Bits) -> Self {
        Self {
            p_state: bits.state(),
        }
    }

    pub fn eval(&self) -> Result<awi::Awi, String> {
        let p_state = self.state();
        eval_thread_local_state(p_state)
    }
}

impl fmt::Debug for EvalAwi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EvalAwi({:?})", self.state())
    }
}

impl<B: AsRef<dag::Bits>> From<B> for EvalAwi {
    fn from(b: B) -> Self {
        Self::from_bits(b.as_ref())
    }
}
