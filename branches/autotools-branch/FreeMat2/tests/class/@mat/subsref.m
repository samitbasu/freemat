function y = subsref(a,s)
switch s.type
case '()'
  ind = s.subs{:};
  y = mat(a.c(ind));
otherwise
  error('Specify value for x as p(x)');
end
