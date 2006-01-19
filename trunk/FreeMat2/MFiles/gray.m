%!
%@Module GRAY Gray Colormap
%@@Section Image
%@@Usage
%Returns a gray colormap.  The syntax for its use is
%@[
%   y = gray
%@]
%@@Example
%Here is an example of an image displayed with the @|gray|
%colormap
%@<
%x = linspace(-1,1,512)'*ones(1,512);
%y = x';
%Z = exp(-(x.^2+y.^2)/0.3);
%image(Z);
%colormap(gray);
%mprint gray1
%@>
%which results in the following image
%@figure gray1
%!
function map = gray(m)
r = linspace(0,1,256)';
map = [r,r,r];
