function docomputeblock(&p,cmds,errorsexpected)
  for i = 1:numel(p.clients)
    docomputeblock(p.clients{i},cmds,errorsexpected);
  end
