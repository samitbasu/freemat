% SPRAND SPRAND Sparse Uniform Random Matrix
% 
% Usage
% 
% Creates a sparse matrix with uniformly distributed random entries (on [0,1]).  The
% syntax for its use is
% 
%   y = sprand(x)
% 
% where x is a sparse matrix, where y is a sparse matrix that has
% random entries where x is nonzero.  The second form specifies the
% size of the matrix and the density
% 
%   y = sprand(m,n,density)
% 
% where m is the number of rows in the output, n is the number of 
% columns in the output, and density (which is between 0 and 1) is
% the density of nonzeros in the resulting matrix.  Note that for very
% high densities the actual density of the output matrix may differ from
% the density you specify.  This difference is a result of the way the
% random entries into the matrix are generated.  If you need a very dense
% random matrix, it is better to generate a full matrix and zero out the 
% entries you do not need.
% SPRAND SPRAND Sparse Uniform Random Matrix
% 
% Usage
% 
% Creates a sparse matrix with uniformly distributed random entries (on [0,1]).  The
% syntax for its use is
% 
%   y = sprand(x)
% 
% where x is a sparse matrix, where y is a sparse matrix that has
% random entries where x is nonzero.  The second form specifies the
% size of the matrix and the density
% 
%   y = sprand(m,n,density)
% 
% where m is the number of rows in the output, n is the number of 
% columns in the output, and density (which is between 0 and 1) is
% the density of nonzeros in the resulting matrix.  Note that for very
% high densities the actual density of the output matrix may differ from
% the density you specify.  This difference is a result of the way the
% random entries into the matrix are generated.  If you need a very dense
% random matrix, it is better to generate a full matrix and zero out the 
% entries you do not need.

% Copyright (c) 2002-2006 Samit Basu
% Licensed under the GPL

function y = sprand(x,n,density)
if (nargin == 1)
  [i,j] = find(x);
  s = rand(size(i));
  y = sparse(i,j,s);
else
  m = round(x(1));
  n = round(n(1));
  density = max(0.0,min(1.0,density(1)));
  cnt = round(density*m*n);
  if (cnt == 0)
    y = sparse(m,n);
    return;
  end
  i = randi(1,m*ones(cnt,1));
  j = randi(1,n*ones(cnt,1));
  A = [i,j];
  A = unique(A,'rows');
  i = A(:,1);
  j = A(:,2);
  s = rand(size(i));
  y = sparse(i,j,s,m,n);
end
 
 
