function [succeeded,failed,errors] = test_core()
succeeded = {};
failed = {};
errors = {};
[succeeded,failed,errors] = runtest('test_assign1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_assign2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_assign3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_assign4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_assign5',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_assign6',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_assign7',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_assign8',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_call1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_call2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_call3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_delete1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_delete2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_delete3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_delete4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_delete5',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_delete6',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_equals1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_diag1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_diag2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_diag3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_diag4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_equals1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_for1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_for2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_for3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_for4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_global1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_if1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_if2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_if3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_image1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_isempty',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_matcat1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_matcat2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_matcat3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_matcat4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_matcat5',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_matcat6',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_persistent1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_plot1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_range1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_range2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_range3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_range4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_range5',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_range6',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_range7',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_range8',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_range9',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_size1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_size2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_size3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_size4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_svd1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_svd2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_svd3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_svd4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_strcmp1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_strcmp2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_struct1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_struct2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_struct3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_struct4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_struct5',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_struct6',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_struct7',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_struct8',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset10',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset11',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset12',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset13',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset14',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset15',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset16',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset17',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset18',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset19',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset5',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset6',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset7',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset8',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_subset9',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_switch1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_switch2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_switch3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_switch4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_switch5',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_switch6',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_switch7',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_test1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_test2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_test3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_test4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_test5',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_typeof1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_typeof2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_typeof3',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_typeof4',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_typeof5',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_while1',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_while2',succeeded,failed,errors);
[succeeded,failed,errors] = runtest('test_while3',succeeded,failed,errors);
