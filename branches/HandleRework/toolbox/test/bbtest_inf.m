% Regression test function (black blox) for FreeMat v4.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_inf
  bbtest_success = 1;
NumErrors = 0;
try
  inf*0

catch
  NumErrors = NumErrors + 1;
end
try
  inf*2

catch
  NumErrors = NumErrors + 1;
end
try
  inf*-2

catch
  NumErrors = NumErrors + 1;
end
try
  inf/inf

catch
  NumErrors = NumErrors + 1;
end
try
  inf/0

catch
  NumErrors = NumErrors + 1;
end
try
  inf/nan

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  uint32(inf)

catch
  NumErrors = NumErrors + 1;
end
try
  complex(inf)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
