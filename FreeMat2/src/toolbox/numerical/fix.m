%!
%@Module FIX Round Towards Zero
%@@Section MATHFUNCTIONS
%@@Usage
%Rounds the argument array towards zero.  The syntax for its use is
%@[
%   y = fix(x)
%@]
%where @|x| is a numeric array.  For positive elements of @|x|, the output
%is the largest integer smaller than @|x|.  For negative elements of @|x|
%the output is the smallest integer larger than @|x|.  For complex @|x|,
%the operation is applied seperately to the real and imaginary parts.
%@@Example
%Here is a simple example of the @|fix| operation on some values
%@<
%a = [-1.8,pi,8,-pi,-0.001,2.3+0.3i]
%fix(a)
%@>
%@@Tests
%@$y1=fix(x1)
%!

% Copyright (c) 2002-2007 Samit Basu
% Licensed under the GPL

function y = fix(x)
  if (isreal(x))
    y = x;
    y(x>0) = floor(x(x>0));
    y(x<0) = ceil(x(x<0));
  else
    y = fix(real(x)) + i*fix(imag(x));
  end
  

