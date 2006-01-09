function h = subplot(varargin)
  m = 1; n = 1; p = 1;
  if (nargin == 1)
    if (isa(varargin{1},'string'))
      str = varargin{1};
      if (length(str) ~= 3), error('Subplot with a string argument requires a string of the type mnp'); end
      m = str(1)-'0';
      n = str(2)-'0';
      p = str(3)-'0';
    else
      axes(varargin{1}(1));
      h = varargin{1}(1);
    end
  else
    if (nargin >= 1), m = int32(varargin{1}(1)); end
    if (nargin >= 2), n = int32(varargin{2}(1)); end
    if (nargin >= 3), p = int32(varargin{3}); p = p(:); end
  end
  row = n+1-int32(idiv(p-1,n)+1);
  col = int32(mod(p-1,n)+1);
  width = 1.0/n;
  height = 1.0/m;
  left = (col-1)*width;
  bottom = (row-1)*height;
  position = [left,bottom,left+width,bottom+height];
  % Get the envelope of the axis
  if (length(p) > 1)
    position = [min(position(:,1:2)),max(position(:,3:4))];
  end
  position = [position(1),position(2),position(3)-position(1),...
	      position(4)-position(2)];
  fig = gcf;
  children = get(fig,'children');
  found = 0;
  deletelist = [];
  for (i=1:length(children))
    if (ishandle(children(i),'axes'))
      outerpos = get(children(i),'outerposition');
      if (all(outerpos==position))
	axes(children(i));
	found = 1;
      elseif (intersects(outerpos,position))
	deletelist = [deletelist,i];
      end
    end
  end
  children(deletelist) = [];
  set(fig,'children',children);
  if (~found)
    h = axes('outerposition',position);
  end

function b = intersects(rect1,rect2)
  nleft = max(rect1(1),rect2(1));
  nbottom = max(rect1(2),rect2(2));
  nright = min(rect1(1)+rect1(3),rect2(1)+rect2(3));
  ntop = min(rect1(2)+rect1(4),rect2(2)+rect2(4));
  if ((nright <= nleft) | (ntop <= nbottom))
    b = 0;
  else
    area = (nright-nleft)*(ntop-nbottom);
    b = area > .01;
  end
