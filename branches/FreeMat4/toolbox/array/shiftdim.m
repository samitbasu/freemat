% SHIFTDIM SHIFTDIM Shift Array Dimensions Function
% 
% Usage
% 
% The shiftdim function is used to shift the dimensions of an array.
% The general syntax for the shiftdim function is
% 
%    y = shiftdim(x,n)
% 
% where x is a multidimensional array, and n is an integer.  If
% n is a positive integer, then shiftdim circularly shifts the 
% dimensions of x to the left, wrapping the dimensions around as 
% necessary.  If n is a negative integer, then shiftdim shifts
% the dimensions of x to the right, introducing singleton dimensions
% as necessary.  In its second form:
% 
%   [y,n] = shiftdim(x)
% 
% the shiftdim function will shift away (to the left) the leading
% singleton dimensions of x until the leading dimension is not
%  a singleton dimension (recall that a singleton dimension p is one for
% which size(x,p) == 1).

% Copyright (c) 2007 Samit Basu
% Licensed under the GPL

function [y,n] = shiftdim(x,p)
  % Compute the size of x
    xsize = size(x);
    if (nargin == 1)
      % Find the first non-singular dimension of x
      p = find(xsize ~= 1,1,'first')-1;
    end
    if (isempty(p) || (p==0))
      % Nothing required
      y = x;
      n = 0;
    elseif (p < 0)
      % For right shifting (negative p), we just reshape the array
      % and introduce extra singleton dimensions
      y = reshape(x,[ones(1,-p),xsize]);
      n = p;
    else
      % Left shifting is modulo the number of dimensions of x
      p = rem(p,ndims(x));
      % For left shifting, (positive p), we can just reshape the
      % array, provided that the leading dimensions are all singletons
      % We have to make sure that the reshape call has at least two
      % dimensions in the shape argument - the number remaining
      % is ndims(x)-p
      if (all(xsize(1:p)==1))
        xsize = xsize((p+1):end);
        if (numel(xsize) < 2), xsize = [xsize,1]; end;
        y = reshape(x,xsize);
        n = p;
      else
        % a permutation is in order
        y = permute(x,[(p+1):ndims(x),1:p]);
        n = p;
      end
    end
    
    
      
      
      
