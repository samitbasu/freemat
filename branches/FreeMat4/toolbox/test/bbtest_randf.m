% Regression test function (black blox) for FreeMat v svn
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_randf
  bbtest_success = 1;
NumErrors = 0;
try
  randf(5*ones(1,9),7)

catch
  NumErrors = NumErrors + 1;
end
try
  randchi(5*ones(1,9))./randchi(7*ones(1,9))

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
