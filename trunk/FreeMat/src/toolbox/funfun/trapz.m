%!
%@Module TRAPZ Trapezoidal Rule Integration
%@@Section NUM 
%@@Usage
% The trapz routine has the following syntax
% @[
%      z = trapz(x,y)
% @]
% where |x| is a dependent vector and |y| an m-by-n matrix equal in
%  at least one dimension to x. (e.g.:|x| = time samples, y = f(t))
% 
% Alternatively, you can enter 
%@[
%     z = trapz(y) 
%@]
% for a unit integration of y. 
%
% If y is a matrix, m must be equal to length(x) (e.g.:
% y must have the same number of rows as x has elements).
% In this case, integrals are taken for each row, returned in a
% resulting vector z of dimension (1,n)
%!

% Copyright (c) 2008 Timothy Cyders
% Licensed under the GPL

function ret = trapz2(x,y)
   
   ret=0;
   
   if nargin == 1
      
      if isvector(x) %%x is vector, unit integration
         for i = 2:length(x)
            ret = ret + (x(i)+x(i-1))/2;
         end
      
      else
         for i = 1:size(x)(2) %% i counts columns
            retj = 0;
            for j =2:size(x)(1) %% j counts rows
               retj = retj +(x(j,i)+x(j-1,i))/2;
            end
            ret(i) = retj;
         end
      end
      
   elseif nargin == 2;
      if isvector(y)
         for i = 2:length(x)
            ret = ret + (x(i) - x(i-1))*(y(i)+y(i-1))*.5;
         end
      else
         for i = 1:size(y)(2) %% i counts columns
            retj = 0;
            for j = 2:length(x) %% j counts rows
               retj = retj + (x(j)-x(j-1))*(y(j,i)+y(j-1,i))*.5;
            end
            ret(i) = retj;
         end
         
   end
end

      