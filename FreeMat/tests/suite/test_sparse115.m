function x = test_sparse115
A = sprandn(10,10,0.3)>0;
B = sprandn(10,10,0.3)>0;
C = A | B;
c = full(A) | full(B);
x = testeq(c,C);
C = A | 0;
c = full(A) | 0;
x = x | testeq(c,C);
C = 1 | A;
c = 1 | full(A);
x = x | testeq(c,C);
