% Regression test function (black blox) for FreeMat v svn
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_threadwait
  bbtest_success = 1;
NumErrors = 0;
try
  a = threadnew;

catch
  NumErrors = NumErrors + 1;
end
try
  threadstart(a,'sleep',0,10);  % start a thread that will sleep for 10

catch
  NumErrors = NumErrors + 1;
end
try
  threadwait(a,2000)            % 2 second wait is not long enough

catch
  NumErrors = NumErrors + 1;
end
try
  threadwait(a,10000)           % 10 second wait is long enough

catch
  NumErrors = NumErrors + 1;
end
try
  threadfree(a)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
