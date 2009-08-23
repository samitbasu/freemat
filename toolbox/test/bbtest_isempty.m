% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_isempty
  bbtest_success = 1;
NumErrors = 0;
try
  a = []

catch
  NumErrors = NumErrors + 1;
end
try
  isempty(a)

catch
  NumErrors = NumErrors + 1;
end
try
  b = 1:3

catch
  NumErrors = NumErrors + 1;
end
try
  isempty(b)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  clear x

catch
  NumErrors = NumErrors + 1;
end
try
  isempty(x)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 1) bbtest_success = 0; return; end
