function gen_wbtest_speye_2(verbose)
  load wbinputs.mat
  error_refs = 0;
  y1 = [];
  try
    z=speye(5000);y1=full(z(1:10,1:10));
  catch
    error_refs = 1;
  end
  if (~error_refs)
  y1_refs = {y1};
  end
  save wbtest_speye_2_ref.mat error_refs  y1_refs 
end
