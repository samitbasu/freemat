% ISSPACE ISSPACE Test for Space Characters in a String
% 
% Usage
% 
% The isspace functions returns a logical array that is 1 
% for characters in the argument string that are spaces, and 
% is a logical 0 for characters in the argument that are not
% spaces.  The syntax for its use is
% 
%    x = isspace(s)
% 
% where s is a string.  A blank character is considered
% a space, newline, tab, carriage return, formfeed, and vertical
% tab.
% ISSPACE ISSPACE Test for Space Characters in a String
% 
% Usage
% 
% The isspace functions returns a logical array that is 1 
% for characters in the argument string that are spaces, and 
% is a logical 0 for characters in the argument that are not
% spaces.  The syntax for its use is
% 
%    x = isspace(s)
% 
% where s is a string.  A blank character is considered
% a space, newline, tab, carriage return, formfeed, and vertical
% tab.

% Copyright (c) 2002-2007 Samit Basu
% Licensed under the GPL

function x = isspace(s)
  s = int32(string(s));
  x = ((s >=9 ) & (s <= 13)) | (s == 32);
  
