% Regression test function (black blox) for FreeMat v svn
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_log2
  bbtest_success = 1;
NumErrors = 0;
try
  x = linspace(1,100);

catch
  NumErrors = NumErrors + 1;
end
try
  plot(x,log2(x))

catch
  NumErrors = NumErrors + 1;
end
try
  xlabel('x');

catch
  NumErrors = NumErrors + 1;
end
try
  ylabel('log2(x)');

catch
  NumErrors = NumErrors + 1;
end
try
  mprint('log2plot');

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
