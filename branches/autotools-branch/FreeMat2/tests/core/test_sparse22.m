function x = test_sparse22
a = randn(1000);
A = sparse(a);
rc = int32(999*rand(7000,1))+1;
cc = int32(999*rand(7000,1))+1;
lc = rc+1000*(cc-1);
b = a(lc);
B = A(lc);
x = testeq(b,B);

