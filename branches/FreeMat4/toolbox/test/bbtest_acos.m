% Regression test function (black blox) for FreeMat v svn
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_acos
  bbtest_success = 1;
NumErrors = 0;
try
  t = linspace(-1,1);

catch
  NumErrors = NumErrors + 1;
end
try
  plot(t,acos(t))

catch
  NumErrors = NumErrors + 1;
end
try
  mprint('acosplot');

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
