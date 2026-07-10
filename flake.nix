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
        overlays = [ fenix.overlays.default ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Latest nightly with the complete component set for warnings and Miri
        rust-nightly = pkgs.fenix.complete.toolchain;

        # A pinned stable toolchain for testing no_std
        rust-pinned = pkgs.fenix.fromToolchainFile {
          file = ./rust-toolchain.toml;
          sha256 = "sha256-h+t2xTBz5yt2YIO+1VMIIGlCU7gyp2LYOFvaV1nwOXU=";
        };

        rust-msrv =
          (pkgs.fenix.toolchainOf {
            channel = "1.86.0";
            sha256 = "sha256-X/4ZBHO3iW0fOenQ3foEvscgAPJYl2abspaBThDOukI=";
          }).toolchain;

        commonTools = with pkgs; [
          just
          cargo-nextest
          cargo-sort
          cargo-machete
          ripgrep
          jq
        ];

        mkShell =
          rust: extraPackages:
          pkgs.mkShell {
            nativeBuildInputs = [
              pkgs.pkg-config
              pkgs.clang-tools
            ];
            hardeningDisable = [ "fortify" ];
            buildInputs = [ rust ] ++ commonTools ++ extraPackages;
          };
      in
      {
        devShells = {
          default = mkShell rust-pinned (
            with pkgs;
            [
              nixd
              nixfmt
            ]
          );
          msrv = mkShell rust-msrv [ ];
          nightly = mkShell rust-nightly [ ];
        };
      }
    );
}
