function gen_wbtest_find_5(verbose)
  load wbinputs.mat
  n_ = numel(wbinputs);
  error_refs = zeros(n_,1);
  y1_refs = cell(n_,1);
  y2_refs = cell(n_,1);
  for loopi=1:n_
    x1 = wbinputs{loopi};
    error_refs(loopi) = 0;
    y1 = [];
    y2 = [];
    try
      [y1,y2]=find(x1);
    catch
      error_refs(loopi) = 1;
    end
    if (~error_refs(loopi))
     y1_refs(loopi) = {y1};
     y2_refs(loopi) = {y2};
    end
  end
  save wbtest_find_5_ref.mat error_refs  y1_refs   y2_refs 
end
