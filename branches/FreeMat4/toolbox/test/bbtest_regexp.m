% Regression test function (black blox) for FreeMat v4.0.0
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_regexp
  bbtest_success = 1;
NumErrors = 0;
try
  [start,stop,tokenExtents,match,tokens,named] = regexp('quick down town zoo','(.)own')

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
