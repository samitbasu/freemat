% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_e
  bbtest_success = 1;
NumErrors = 0;
try
  e

catch
  NumErrors = NumErrors + 1;
end
try
  log(e)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
