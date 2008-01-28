%!
%@Module DEC2BIN Convert Decimal to Binary String
%@@Section TYPECAST
%@@USAGE
%Converts an integer to a binary string.  The syntax for its
%use is
%@[
%   y = dec2bin(x,n)
%@]
%where @|x| is the positive integer, and @|n| is the number of
%bits to use in the representation.  Alternately, if you leave
%@|n| unspecified, 
%@[
%   y = dec2bin(x)
%@]
%the minimum number of bits needed to represent @|x| are used.
%If @|x| is a vector, then the resulting @|y| is a character
%matrix.
%@@Example
%Here we convert some numbers to bits
%@<
%dec2bin(56)
%dec2bin(1039456)
%dec2bin([63,73,32],5)
%@>
%!

% Copyright (c) 2002-2006 Samit Basu

function t = dec2bin(x,n)
  x = x(:); 
  if (~exist('n') && max(x) > 0)
    n = ceil(log2(max(x)));
  elseif (~exist('n'))
    t = zeros(size(x));
    return;
  elseif (max(x) == 0)
    t = zeros(size(x));
    return;
  end
  t = string(int2bin(x,n)+'0');
