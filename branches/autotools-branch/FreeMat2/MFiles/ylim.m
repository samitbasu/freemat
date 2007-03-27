%!
%@Module YLIM Adjust Y Limits of plot
%@@Section HANDLE
%@@Usage
%There are several ways to use @|ylim| to adjust the y limits of
%a plot.  The x-axis retains its current limits.  The four
%syntaxes are
%@[
%   ylim([lo,hi])   
%   ylim(lo,hi)
%   ylim('auto')
%   ylim auto
%@]
%where in the first two forms, the new y-limits on the plot are
%@|[lo,hi]|.  In the second two forms, the axes limits are 
%automatically selected by FreeMat.
%@@Example
%Here is an example of using @|ylim| to zoom in on the y axis of a
%plot without changing the x limits.  First, the plot with default
%limits
%@<
%x = linspace(-1,1);
%y = sin(2*pi*x);
%plot(x,y,'r-');
%mprint ylim1
%@>
%which results in
%@figure ylim1
%Next, we zoom in on the plot using the @|ylim| function
%@<
%plot(x,y,'r-')
%ylim(-0.2,0.2)
%mprint ylim2
%@>
%which results in
%@figure ylim2
%!

% Copyright (c) 2002-2006 Samit Basu

%Copyright (c) 2004,2005 Brian Yanoff, Samit Basu
function ylim(lim1, lim2)
  if isa(lim1,'string') && strcmp(lim1,'auto')
     set(gca,'ylimmode','auto');
     return;
  elseif isa(lim1,'string')
      error('do not understand arguments to ylim function');
  end

  if prod(size(lim1))>1
    lo_lim = lim1(1);
    hi_lim = lim1(2);
  elseif ~isset('lim2')
    error('ylim requires a 2-vector or two scalar parameters');
  elseif prod(size(lim1))==1
    lo_lim = lim1;
    hi_lim = lim2(1);
  end
  set(gca,'ylim',[lo_lim,hi_lim]);

  