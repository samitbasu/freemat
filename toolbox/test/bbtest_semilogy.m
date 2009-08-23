% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_semilogy
  bbtest_success = 1;
NumErrors = 0;
try
  x = linspace(0,2);

catch
  NumErrors = NumErrors + 1;
end
try
  y = 10.0.^x;

catch
  NumErrors = NumErrors + 1;
end
try
  plot(x,y,'r-');

catch
  NumErrors = NumErrors + 1;
end
try
  mprint semilogy1

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  semilogy(x,y,'r-');

catch
  NumErrors = NumErrors + 1;
end
try
  mprint semilogy2

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
