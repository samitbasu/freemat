% Regression test function (black blox) for FreeMat v svn
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_csvwrite
  bbtest_success = 1;
NumErrors = 0;
try
  x = [1,2,3;5,6,7]

catch
  NumErrors = NumErrors + 1;
end
try
  csvwrite('csvwrite.csv',x)

catch
  NumErrors = NumErrors + 1;
end
try
  csvread('csvwrite.csv')

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
NumErrors = 0;
try
  csvwrite('csvwrite.csv',x,1,2)

catch
  NumErrors = NumErrors + 1;
end
try
  csvread('csvwrite.csv')

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
