% test plot of an empty argument
function test_val = test_plot1
a = [];
try
  % If this causes a segfault, it won't be caught.
  plot(a,a);
catch
end
closeplot all;
test_val = 1;
