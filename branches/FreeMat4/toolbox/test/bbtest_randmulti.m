% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_randmulti
  bbtest_success = 1;
NumErrors = 0;
try
  randmulti(10000,[0.4999,0.4999,0.0002])

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
