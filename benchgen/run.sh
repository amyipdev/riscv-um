#!/usr/bin/env sh

gcc -o benchgen benchgen.c
./benchgen
riscv64-unknown-linux-gnu-as -march=rv64imafd reg_bench_gen.s -o rb.o
riscv64-unknown-linux-gnu-ld rb.o -o rb
qemu-riscv64 ./rb
