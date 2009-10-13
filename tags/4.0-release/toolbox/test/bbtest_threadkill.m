% Regression test function (black blox) for FreeMat v4.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_threadkill
  bbtest_success = 1;
NumErrors = 0;
try
  a = threadnew;

catch
  NumErrors = NumErrors + 1;
end
try
  global count                   % register the global variable count

catch
  NumErrors = NumErrors + 1;
end
try
  count = 0;

catch
  NumErrors = NumErrors + 1;
end
try
  threadstart(a,'freecount',0)   % start the thread

catch
  NumErrors = NumErrors + 1;
end
try
  count                          % it is counting

catch
  NumErrors = NumErrors + 1;
end
try
  sleep(1)                       % Wait a bit

catch
  NumErrors = NumErrors + 1;
end
try
  count                          % it is still counting

catch
  NumErrors = NumErrors + 1;
end
try
  threadkill(a)                  % kill the counter

catch
  NumErrors = NumErrors + 1;
end
try
  threadwait(a,1000)             % wait for it to finish

catch
  NumErrors = NumErrors + 1;
end
try
  count                          % The count will no longer increase

catch
  NumErrors = NumErrors + 1;
end
try
  sleep(1)

catch
  NumErrors = NumErrors + 1;
end
try
  count

catch
  NumErrors = NumErrors + 1;
end
try
  threadfree(a)

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
