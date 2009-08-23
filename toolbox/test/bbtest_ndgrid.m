% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_ndgrid
  bbtest_success = 1;
NumErrors = 0;
try
  [a,b] = ndgrid(1:2,3:5)

catch
  NumErrors = NumErrors + 1;
end
try
  [a,b,c] = ndgrid(1:2,3:5,0:1)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  [a,b,c] = ndgrid(1:3)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
