#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define OUTPUT_FILE "./reg_bench_gen.s"
#define INSTRUCTION_COUNT 2000000

// Generate a variety of RISC-V instructions
void generate_riscv_instructions(FILE *output) {
    unsigned int reg_a = 0;
    unsigned int reg_b = 1;
    unsigned int reg_c = 2;
    unsigned int reg_d = 3;
    unsigned int reg_temp;

    // Write assembly preamble
    fprintf(output, "\t.section .data\n");
    fprintf(output, "\t.section .text\n");
    fprintf(output, "\t.globl _start\n\n");
    fprintf(output, "_start:\n");
    fprintf(output, "\tli t0,8745425\n\tli t1,2413112\n\tli t2,51124341\n\tli t3,991232131\n");

    for (unsigned int i = 0; i < INSTRUCTION_COUNT; i++) {
        switch (i % 3) {
            case 0:
                // Add two registers
                fprintf(output, "\tadd t%d, t%d, t%d\n", reg_a, reg_b, reg_c);
                break;
            case 1:
                // Subtract two registers
                fprintf(output, "\tsub t%d, t%d, t%d\n", reg_b, reg_c, reg_d);
                break;
            case 2:
                // Shift left logical
                fprintf(output, "\tsll t%d, t%d, t%d\n", reg_a, reg_b, reg_c);
                break;
        }

        // Rotate register assignments for variety
        reg_temp = reg_a;
        reg_a = reg_b;
        reg_b = reg_c;
        reg_c = reg_d;
        reg_d = reg_temp;
    }

    // Validate results
    //fprintf(output, "\tli t4, 18260581967612606931\n");
    fprintf(output, "\tli t4, 8697740129876948287\n");
    fprintf(output, "\tli a0,1\n\tbne t0, t4, validation_failed\n");
    fprintf(output, "\tli t4, 0\n");
    fprintf(output, "\tli a0,2\n\tbne t1, t4, validation_failed\n");
    //fprintf(output, "\tli t4, 186162106096944685\n");
    fprintf(output, "\tli t4, 9749003943832603329\n");
    fprintf(output, "\tli a0,3\n\tbne t2, t4, validation_failed\n");
    //fprintf(output, "\tli t4, 6046945345489228955\n");
    fprintf(output, "\tli t4, 18220595702735330224\n");
    fprintf(output, "\tli a0,4\n\tbne t3, t4, validation_failed\n");
    fprintf(output, "\tli a7, 93\n\tli a0,0\n\tecall\n\n"); // Exit with success

    fprintf(output, "validation_failed:\n");
    fprintf(output, "\tli a7, 93\n\tecall\n\n"); // Exit with error
}

int main() {
    FILE *output = fopen(OUTPUT_FILE, "w");
    if (!output) {
        perror("Failed to open output file");
        return EXIT_FAILURE;
    }

    generate_riscv_instructions(output);

    fclose(output);
    printf("RISC-V assembly written to %s\n", OUTPUT_FILE);
    return EXIT_SUCCESS;
}
