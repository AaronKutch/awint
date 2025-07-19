alias t := test
alias r := run
alias c := check

ALL_FEATURES := "--features=std,zeroize_support,rand_support,serde_support,dag,try_support,debug"

quick:
  cargo fmt
  cargo clippy --all --all-targets {{ALL_FEATURES}} -- -D clippy::all

check:
  cargo check
  cargo clippy --all --all-targets -- -D clippy::all
  cargo doc

test *ARGS:
  cargo nextest run {{ARGS}}

bench *ARGS:
  cargo bench -p testcrate {{ARGS}}

run *ARGS:
  cargo r --bin {{ARGS}}

miri *ARGS:
  MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-strict-provenance" cargo miri test {{ALL_FEATURES}} {{ARGS}}

clean:
  cargo clean
