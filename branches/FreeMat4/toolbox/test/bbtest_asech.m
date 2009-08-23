% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_asech
  bbtest_success = 1;
NumErrors = 0;
try
  x1 = -20:.01:-1;

catch
  NumErrors = NumErrors + 1;
end
try
  x2 = 1:.01:20;

catch
  NumErrors = NumErrors + 1;
end
try
  plot(x1,imag(asech(x1)),x2,imag(asech(x2))); grid('on');

catch
  NumErrors = NumErrors + 1;
end
try
  mprint('asechplot');

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
