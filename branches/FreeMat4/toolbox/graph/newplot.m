% NEWPLOT NEWPLOT Get Handle For Next Plot
% 
% Usage
% 
% Returns the handle for the next plot operation.  The general
% syntax for its use is
% 
%   h = newplot
% 
% This routine checks the nextplot properties of the current
% figure and axes to see if they are set to replace or not. If
% the figures nextplot property is set to replace, the current
% figure is cleared.  If the axes nextplot property is set to
% replace then the axes are cleared for the next operation.  
% NEWPLOT NEWPLOT Get Handle For Next Plot
% 
% Usage
% 
% Returns the handle for the next plot operation.  The general
% syntax for its use is
% 
%   h = newplot
% 
% This routine checks the nextplot properties of the current
% figure and axes to see if they are set to replace or not. If
% the figures nextplot property is set to replace, the current
% figure is cleared.  If the axes nextplot property is set to
% replace then the axes are cleared for the next operation.  
% NEWPLOT NEWPLOT Get Handle For Next Plot
% 
% Usage
% 
% Returns the handle for the next plot operation.  The general
% syntax for its use is
% 
%   h = newplot
% 
% This routine checks the nextplot properties of the current
% figure and axes to see if they are set to replace or not. If
% the figures nextplot property is set to replace, the current
% figure is cleared.  If the axes nextplot property is set to
% replace then the axes are cleared for the next operation.  
% NEWPLOT NEWPLOT Get Handle For Next Plot
% 
% Usage
% 
% Returns the handle for the next plot operation.  The general
% syntax for its use is
% 
%   h = newplot
% 
% This routine checks the nextplot properties of the current
% figure and axes to see if they are set to replace or not. If
% the figures nextplot property is set to replace, the current
% figure is cleared.  If the axes nextplot property is set to
% replace then the axes are cleared for the next operation.  

% Copyright (c) 2002-2006 Samit Basu
% Licensed under the GPL

function h = newplot
fig = gcf;
fg_mode = get(fig,'nextplot');
if (strcmp(fg_mode,'replace'))
     clf;
end
ax = gca;
ax_mode = get(ax,'nextplot');
if (strcmp(ax_mode,'replace'))
     cla;
end
h = gca;



