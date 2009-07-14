% POLYINT POLYINT Polynomial Coefficient Integration
% 
% Usage
% 
%  The polyint function returns the polynomial coefficients resulting
%  from integration of polynomial p. The syntax for its use is either
% 
%  pint = polyint(p,k)
% 
%  or, for a default k = 0,
% 
%  pint = polyint(p);
% 
%  where p is a vector of polynomial coefficients assumed to be in
%  decreasing degree and k is the integration constant.
%  Contributed by Paulo Xavier Candeias under GPL
% POLYINT POLYINT Polynomial Coefficient Integration
% 
% Usage
% 
%  The polyint function returns the polynomial coefficients resulting
%  from integration of polynomial p. The syntax for its use is either
% 
%  pint = polyint(p,k)
% 
%  or, for a default k = 0,
% 
%  pint = polyint(p);
% 
%  where p is a vector of polynomial coefficients assumed to be in
%  decreasing degree and k is the integration constant.
%  Contributed by Paulo Xavier Candeias under GPL
% POLYINT POLYINT Polynomial Coefficient Integration
% 
% Usage
% 
%  The polyint function returns the polynomial coefficients resulting
%  from integration of polynomial p. The syntax for its use is either
% 
%  pint = polyint(p,k)
% 
%  or, for a default k = 0,
% 
%  pint = polyint(p);
% 
%  where p is a vector of polynomial coefficients assumed to be in
%  decreasing degree and k is the integration constant.
%  Contributed by Paulo Xavier Candeias under GPL
% POLYINT POLYINT Polynomial Coefficient Integration
% 
% Usage
% 
%  The polyint function returns the polynomial coefficients resulting
%  from integration of polynomial p. The syntax for its use is either
% 
%  pint = polyint(p,k)
% 
%  or, for a default k = 0,
% 
%  pint = polyint(p);
% 
%  where p is a vector of polynomial coefficients assumed to be in
%  decreasing degree and k is the integration constant.
%  Contributed by Paulo Xavier Candeias under GPL
function pint = polyint(p,k)
   if nargin < 1
      error('wrong use (see help polyint)');
   elseif nargin < 2
      k = 0;
   end
   pint = [(p(:).')./(length(p):-1:1),k];
