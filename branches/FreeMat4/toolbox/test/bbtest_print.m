% Regression test function (black blox) for FreeMat v4.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_print
  bbtest_success = 1;
NumErrors = 0;
try
  x = linspace(-1,1);

catch
  NumErrors = NumErrors + 1;
end
try
  y = cos(5*pi*x);

catch
  NumErrors = NumErrors + 1;
end
try
  plot(x,y,'r-');

catch
  NumErrors = NumErrors + 1;
end
try
  print('printfig1.jpg')

catch
  NumErrors = NumErrors + 1;
end
try
  print('printfig1.png')

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
