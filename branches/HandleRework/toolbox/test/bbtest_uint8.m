% Regression test function (black blox) for FreeMat v4.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_uint8
  bbtest_success = 1;
NumErrors = 0;
try
  uint8(200)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  uint8(400)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  uint8(-100)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  uint8(pi)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  uint8(5+2*i)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  uint8('helo')

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  uint8({4})

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 1) bbtest_success = 0; return; end
