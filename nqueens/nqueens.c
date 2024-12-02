// Derived from the GeekssforGeeks n-queens algorithm

int ld[30] = { 0 };
int rd[30] = { 0 };
int cl[30] = { 0 };

#define N 4

void write_stdout(char *s, int n) {
    asm volatile (
        "addi a7,x0,64\n\t"
        "addi a0,x0,1\n\t"
        "addi a1,%0,0\n\t"
        "addi a2,%1,0\n\t"
        "ecall"
        :
        : "r"(s), "r"(n)
        : "a0", "a1", "a2", "a7"
    );
}

// A utility function to print solution
void printSolution(int board[N][N]) {
    for (int i = 0; i < N; i++) {
        for (int j = 0; j < N; j++) {
            char x[] = " . ";
            if (board[i][j])
                x[1] = 'Q';
            write_stdout(x, 4);
        }
        write_stdout("\n", 2);
    }
}

int solveNQUtil(int board[N][N], int col) {
    if (col >= N)
        return 1;
    for (int i = 0; i < N; i++) {
        if ((ld[i - col + N - 1] != 1 && rd[i + col] != 1)
            && cl[i] != 1) {
            board[i][col] = 1;
            ld[i - col + N - 1] = rd[i + col] = cl[i] = 1;
            if (solveNQUtil(board, col + 1))
                return 1;
            board[i][col] = 0;
            ld[i - col + N - 1] = rd[i + col] = cl[i] = 0;
        }
    }
    return 0;
}

int solveNQ() {
    //int** board = (int**)malloc(n * sizeof(int*));
    int board[N][N];

    for (int i = 0; i < N; i++)
        for (int j = 0; j < N; j++)
            board[i][j] = 0;

    if (solveNQUtil(board, 0) == 0) {
        write_stdout("Solution does not exist", 24);
        return 0;
    }

    printSolution(board);
    return 1;
}

void _start() {
    solveNQ();
    asm volatile (
        "addi a0,x0,0\n\t"
        "addi a7,x0,93\n\t"
        "ecall"
    );
}
