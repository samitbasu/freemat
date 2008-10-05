%!
%@Module ROOTS Find Roots of Polynomial
%@@Section CURVEFIT
%@@Usage
%The @|roots| routine will return a column vector containing the
%roots of a polynomial.  The general syntax is
%@[
%   z = roots(p)
%@]
%where @|p| is a vector containing the coefficients of the polynomial
%ordered in descending powers.  
%@@Function Internals
%Given a vector 
%\[
%   [p_1, p_2, \dots p_n]
%\]
%which describes a polynomial
%\[
%   p_1 x^{n-1} + p_2 x^{n-2} + \dots + p_n
%\]
%we construct the companion matrix (which has a characteristic polynomial
%matching the polynomial described by @|p|), and then find the eigenvalues
%of it (which are the roots of its characteristic polynomial), and
%which are also the roots of the polynomial of interest.  This technique
%for finding the roots is described in the help page for @|roots| on the Mathworks
%website.
%@@Example
%Here is an example of finding the roots to the polynomial
%\[
%   x^3 - 6x^2 - 72x - 27
%\]
%@<
%roots([1 -6 -72 -27])
%@>
%@@Tests
%@$exact|y1=roots(x1)
%!

% Copyright (c) 2002-2007 Samit Basu
% Licensed under the GPL

function z = roots(p)
  if(any(isnan(p) | isinf(p)))
     error('Input to ROOTS must not contain NaN or Inf.');
  end
  if (isempty(p))
    z = zeros(0,1);
    return;
  end
  while(any(isinf(p./p(1))))
    p=p(2:end);
  end
  if (numel(p) <= 1) 
    z = zeros(0,1);
    return;
  end
  p = vec(p);
  n = numel(p)-1;
  o = ones(n-1,1);
  if (isa(p,'single'))
    o = single(o);
  end
  A = diag(o,-1);
  A(1,:) = -p(2:n+1)./p(1);
  z = eig(A);
  
