% Copyright (c) 2002, 2003 Samit Basu
%
% Permission is hereby granted, free of charge, to any person obtaining a 
% copy of this software and associated documentation files (the "Software"), 
% to deal in the Software without restriction, including without limitation 
% the rights to use, copy, modify, merge, publish, distribute, sublicense, 
% and/or sell copies of the Software, and to permit persons to whom the 
% Software is furnished to do so, subject to the following conditions:
%
% The above copyright notice and this permission notice shall be included 
% in all copies or substantial portions of the Software.
%
% THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS 
% OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, 
% FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL 
% THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER 
% LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
% FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER 
% DEALINGS IN THE SOFTWARE.

function test_val = test_resize

% Check a normal resize in 2D
a = [];
a = [1,2;3,4];
a(3,3) = 1;

test_val(1) = test(a == [1,2,0;3,4,0;0,0,1]);

% Check a scalar -> vector resize
a = 1;
a(3) = 1;
test_val(2) = test(a == [1,0,1]);

% Check a row-vector -> vector resize
a = [1 2];
a(3) = 1;
test_val(3) = test(a == [1,2,1]);

% Check a column-vector -> vector resize
a = [1;2];
a(3) = 1;
test_val(4) = test(a == [1;2;1]);

% Check an n-ary -> vector resize
a = 0;
a(2,1,2) = 0;
a(5) = 1;
test_val(5) = test(a == [0,0,0,0,1]);


