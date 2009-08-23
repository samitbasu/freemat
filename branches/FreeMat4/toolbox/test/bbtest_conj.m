% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_conj
  bbtest_success = 1;
NumErrors = 0;
try
  conj(3+4*i)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  conj([2,3,4])

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  conj([2.0+3.0*i,i])

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
