{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      crane,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rust-toolchain = pkgs.rust-bin.stable.latest;
        craneLib = (crane.mkLib pkgs).overrideToolchain (p: rust-toolchain.minimal);
        buildDeps = [
          pkgs.pkg-config
          pkgs.alsa-lib
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rust-toolchain.default
            rust-toolchain.rust-src
            rust-toolchain.rust-analyzer
          ];
          nativeBuildInputs = buildDeps;
          RUST_SRC_PATH = "${rust-toolchain.rust-src}/lib/rustlib/src/rust/library";
        };

        packages.default = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./.;
          nativeBuildInputs = buildDeps;
        };
      }
    );
}
