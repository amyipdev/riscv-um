with import <nixpkgs> {};

let
  fenix = import (fetchTarball "https://github.com/nix-community/fenix/archive/main.tar.gz") {};
  combined = fenix.combine [
    fenix.complete.toolchain
    fenix.targets.riscv64gc-unknown-linux-gnu.latest.toolchain
    #fenix.targets.riscv64gc-unknown-none-elf.latest.rust-std
  ];
  riscv-toolchain = import <nixpkgs> {
    localSystem = "${system}";
    crossSystem = {
      config = "riscv64-unknown-linux-gnu";
    };
  };
in

stdenv.mkDerivation {
  name = "riscv-um-env";
  nativeBuildInputs = [
    #fenix.combine [
      #fenix.complete.toolchain
      #fenix.targets.riscv32i-unknown-none-elf.latest.toolchain
    #]
    combined
    qemu
    strace
    gdb
    riscv-toolchain.buildPackages.gcc
    # qemu-user doesn't exist until 24.11
    # pkg-config openssl
  ];
  shellHook = ''
    alias cargo-rv64="RUSTFLAGS=\"-C linker=riscv64-unknown-linux-gnu-gcc\" cargo"
  '';
}
