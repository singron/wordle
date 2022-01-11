{
  sources ? import ./nix/sources.nix,
  pkgs ? import <nixpkgs> {
    overlays = [ (import sources.rust-overlay) ];
  },
}:
pkgs.mkShell {
  CARGO_MANIFEST_DIR=toString ./.;
  buildInputs = with pkgs; [
    (rust-bin.nightly.latest.default.override {
      extensions = ["rust-src"];
    })
    cargo rustc rust-analyzer rustfmt clippy
    cargo-flamegraph cargo-asm
    niv
  ];
}
