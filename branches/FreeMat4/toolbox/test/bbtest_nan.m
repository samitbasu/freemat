% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_nan
  bbtest_success = 1;
NumErrors = 0;
try
  nan*0

catch
  NumErrors = NumErrors + 1;
end
try
  nan-nan

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  uint32(nan)

catch
  NumErrors = NumErrors + 1;
end
try
  complex(nan)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
