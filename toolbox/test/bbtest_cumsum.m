% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_cumsum
  bbtest_success = 1;
NumErrors = 0;
try
  A = [5,1,3;3,2,1;0,3,1]

catch
  NumErrors = NumErrors + 1;
end
try
  cumsum(A)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  cumsum(A,2)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  B(:,:,1) = [5,2;8,9];

catch
  NumErrors = NumErrors + 1;
end
try
  B(:,:,2) = [1,0;3,0]

catch
  NumErrors = NumErrors + 1;
end
try
  cumsum(B,3)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
