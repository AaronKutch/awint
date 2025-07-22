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
  cargo nextest run {{ALL_FEATURES}} {{ARGS}}

test_all *ARGS:
  cargo nextest run {{ALL_FEATURES}},const_support {{ARGS}}
  cargo t --doc {{ALL_FEATURES}},const_support {{ARGS}}

test_stable *ARGS:
  cargo +nightly-2023-04-14 t {{ALL_FEATURES}} {{ARGS}}
  cargo +nightly-2023-04-14 t --doc {{ALL_FEATURES}} {{ARGS}}

bench *ARGS:
  cargo bench -p testcrate {{ARGS}}

run *ARGS:
  cargo r --bin {{ARGS}}

miri *ARGS:
  MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-strict-provenance" cargo miri test {{ALL_FEATURES}} {{ARGS}}

clean:
  cargo clean
