with import <nixpkgs> {};

clangStdenv.mkDerivation {
  name = "serverize";
  buildInputs = [
    rustChannels.stable.rust
    rustChannels.stable.rust-src rustracer
  ];
  RUST_SRC_PATH="${rustChannels.stable.rust-src}/lib/rustlib/src/rust/src";
}
