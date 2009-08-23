% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_setpath
  bbtest_success = 1;
NumErrors = 0;
try
  getpath

catch
  NumErrors = NumErrors + 1;
end
try
  setpath('/usr/local/FreeMat/MFiles:/localhome/basu/MFiles')

catch
  NumErrors = NumErrors + 1;
end
try
  getpath

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
