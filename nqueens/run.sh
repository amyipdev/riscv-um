#!/usr/bin/env sh

riscv64-unknown-linux-gnu-gcc -march=rv64imafd -S -fno-asynchronous-unwind-tables -fno-stack-protector -fno-exceptions -fomit-frame-pointer -fno-dwarf2-cfi-asm -fno-tree-loop-distribute-patterns -Wall -Wextra -Werror -nostdlib -O3 -g0 nqueens.c
riscv64-unknown-linux-gnu-as -march=rv64imafd nqueens.s -o nqueens.o
riscv64-unknown-linux-gnu-ld nqueens.o -o nqueens
qemu-riscv64 ./nqueens
