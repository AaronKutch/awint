pub mod basic_state_epoch;
pub mod epoch;
mod error;
mod eval;
mod lineage;
mod noop;
mod op;
mod state;

use awint_macro_internals::triple_arena::ptr_struct;
pub use error::EvalError;
pub use eval::*;
pub use lineage::Lineage;
pub use noop::*;
pub use op::*;
pub use state::*;

ptr_struct!(PNote);
