with import <nixpkgs> {};

let
  src = fetchFromGitHub {
    owner = "mozilla";
    repo = "nixpkgs-mozilla";
    rev = "9b11a87c0cc54e308fa83aac5b4ee1816d5418a2";
    sha256 = "+gi59LRWRQmwROrmE1E2b3mtocwueCQqZ60CwLG+gbg=";
  };

in

with import "${src.out}/rust-overlay.nix" pkgs pkgs;

stdenv.mkDerivation {
  name = "riscv-um-env";
  buildInputs = [
    latest.rustChannels.nightly.rust

    # qemu-user doesn't exist until 24.11
    qemu

    # pkg-config openssl
  ];
}
