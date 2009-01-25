% Regression test function (black blox) for FreeMat v svn
% This function is autogenerated by helpgen.
function bbtest_success = bbtest_try
  bbtest_success = 1;
NumErrors = 0;
try
  read_file('this_filename_is_invalid')

catch
  NumErrors = NumErrors + 1;
end
try
  fp = fopen('test_text.txt','w');

catch
  NumErrors = NumErrors + 1;
end
try
  fprintf(fp,'a line of text\n');

catch
  NumErrors = NumErrors + 1;
end
try
  fclose(fp);

catch
  NumErrors = NumErrors + 1;
end
try
  read_file('test_text.txt')

catch
  NumErrors = NumErrors + 1;
end
if (NumErrors ~= 0) bbtest_success = 0; return; end
