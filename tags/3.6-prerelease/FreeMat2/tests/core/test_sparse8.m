function x = test_sparse8
ar = [0,0,2,0,0;0,1,0,1,0;0,0,0,1,0];
ai = [1,0,3,0,0;0,2,3,-1,3;0,1,0,4,3];
a = complex(ar+i*ai);
A = sparse(a);
b = [1,4,5;2,5,15;3,6,11];
C = A(b);
c = a(b);
x = testeq(c,C);

