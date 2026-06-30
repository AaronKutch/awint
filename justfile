# The empty default uses whatever cargo is already active, which can come from the nix shell or
# rustup default
#
# - Nix: `nix develop .#nightly -c just check` (or `.#msrv`, or nothing for default pinned)
# - rustup: `just toolchain=nightly check` (or `toolchain=1.85`, etc.)
toolchain := ""
cargo := if toolchain == "" { "cargo" } else { "cargo +" + toolchain }

alias c := check
alias t := test
alias r := run

ALL_FEATURES := "--features=std,zeroize_support,rand_support,serde_support,dag,try_support,debug"

quick:
  {{cargo}} fmt
  {{cargo}} clippy --all --all-targets {{ALL_FEATURES}} -- -D clippy::all

fmt:
  {{cargo}} sort -w
  {{cargo}} fmt

check:
  {{cargo}} check
  {{cargo}} clippy --all --all-targets -- -D clippy::all

test *ARGS:
  {{cargo}} nextest run {{ALL_FEATURES}} {{ARGS}}

test_all *ARGS:
  {{cargo}} sort -cw
  {{cargo}} doc --no-deps
  {{cargo}} nextest run {{ALL_FEATURES}},const_support {{ARGS}}
  {{cargo}} t --doc {{ALL_FEATURES}},const_support {{ARGS}}

# Needs to be run with the MSRV toolchain
test_for_msrv *ARGS:
  {{cargo}} t {{ALL_FEATURES}} {{ARGS}}
  {{cargo}} t --doc {{ALL_FEATURES}} {{ARGS}}

bench *ARGS:
  {{cargo}} bench -p testcrate {{ARGS}}

run *ARGS:
  {{cargo}} r --bin {{ARGS}}

miri *ARGS:
  MIRIFLAGS="-Zmiri-tree-borrows -Zmiri-strict-provenance" {{cargo}} miri test {{ALL_FEATURES}} {{ARGS}}

clean:
  {{cargo}} clean

# Print the nix shell's PATH, for VSCode for instance you can add this to get rust-analyzer to work:
# `"rust-analyzer.cargo.extraEnv": {"NIX_PROFILES": "/nix/var/nix/profiles/default ${userHome}/.nix-profile", "PATH": "..."},`
ra_path:
  nix develop .#nightly --command printenv PATH
