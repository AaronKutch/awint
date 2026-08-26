{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      fenix,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        fenix-pkgs = fenix.packages.${system};

        # Latest nightly with the complete component set for warnings and Miri
        rust-nightly = fenix-pkgs.combine [
          fenix-pkgs.complete.toolchain
          fenix-pkgs.targets.riscv32i-unknown-none-elf.latest.rust-std
        ];

        # A known good pinned stable with needed components
        rust-pinned = fenix-pkgs.fromToolchainFile {
          file = ./pinned-toolchain.toml;
          sha256 = "sha256-A1abGIbOtcBSdrUMhDGrER3pRM1hQP4fp9gh3Y4PKc8=";
        };

        # Uses the crate MSRV with minimal profile
        msrv = (builtins.fromTOML (builtins.readFile ./awint_internals/Cargo.toml)).package.rust-version;
        rust-msrv =
          (fenix-pkgs.toolchainOf {
            channel = msrv;
            sha256 = "sha256-Qxt8XAuaUR2OMdKbN4u8dBJOhSHxS+uS06Wl9+flVEk=";
          }).minimalToolchain;

        commonTools = with pkgs; [
          nixfmt
          just
          ripgrep
          jq
          cargo-sort
          cargo-machete
          cargo-nextest
          cargo-show-asm
        ];

        # NOTE: `packages` is the field for things that just need to be on the `PATH`,
        # `buildInputs` is for libraries that get linked against. There is no `pkg-config` or
        # `hardeningDisable` because nothing in the dependency tree compiles any C.
        mkShell = rust: pkgs.mkShell { packages = [ rust ] ++ commonTools; };
      in
      {
        devShells = {
          default = mkShell rust-pinned;
          msrv = mkShell rust-msrv;
          nightly = mkShell rust-nightly;
        };

        # run `nix fmt` to format all nix files
        formatter = pkgs.nixfmt-tree;
      }
    );
}
