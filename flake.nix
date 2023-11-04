# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay, }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };
      toolchain = pkgs.rust-bin.fromRustupToolchainFile ./toolchain.toml;
    in {
      devShells.${system}.default = let
        build-deps = with pkgs; [
          pkg-config
          openssl
        ];
      in pkgs.mkShell.override {
        stdenv = pkgs.stdenvAdapters.useMoldLinker pkgs.stdenv;
      } {
        packages = build-deps ++ [
          toolchain
          pkgs.cargo-bloat
          pkgs.cargo-unused-features
          pkgs.rust-analyzer-unwrapped
          pkgs.cargo-watch
          pkgs.cargo-sort
          pkgs.cargo-machete
          pkgs.cargo-depgraph
          pkgs.cargo-limit
          pkgs.pre-commit
          pkgs.nodejs_18
        ];

        shellHook = ''
          pre-commit install
        '';

        RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
        NIX_LD = pkgs.runCommand "ld.so" { } ''
          ln -s "$(cat '${pkgs.stdenv.cc}/nix-support/dynamic-linker')" $out
        '';
        LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath build-deps}";

      };
    };
}
