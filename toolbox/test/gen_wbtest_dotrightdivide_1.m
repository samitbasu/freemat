function gen_wbtest_dotrightdivide_1(verbose)
  load reference/wbinputs.mat
  n_ = numel(wbinputs);
  error_refs = zeros(n_,n_);
  y1_refs = cell(n_,n_);
  for loopi=1:n_
    for loopj=1:n_
      x1 = wbinputs{loopi};
      x2 = wbinputs{loopj};
      y1 = [];
      try
        y1=x1./x2;
      catch
        error_refs(loopi,loopj) = 1;
      end
      if (~error_refs(loopi,loopj))
       y1_refs(loopi,loopj) = {y1};
      end
    end
  end
  save reference/wbtest_dotrightdivide_1_ref.mat error_refs  y1_refs 
end
