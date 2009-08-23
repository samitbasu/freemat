% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_randbin
  bbtest_success = 1;
NumErrors = 0;
try
  randbin(100,.1*ones(1,10))

catch
  NumErrors = NumErrors + 1;
end
try
  sum(rand(100,10)<0.1)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
