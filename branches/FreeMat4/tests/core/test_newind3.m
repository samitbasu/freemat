printf('line 1\n');
a(1,40,50).foo = 'green';
printf('line 2\n');
p = a(1,3,5).foo;
printf('line 2bn');
q = a(1,3,5).foo;
printf('line 2\n');
p = a(:,3,5).foo;
printf('line 2bn');
q = a(:,3,5).foo;
printf('line 3\n');
p = 4;
printf('line 4\n');
a(10,40,50).foo = 'green';
printf('line 5\n');
p = a(1,:,5).foo;
printf('line 6\n');
p = 4;
%
%a(10,40,50).hoo = 'green';
%p = a(:,:,5);
%p = 4;
%p = a(1,4,3);
%p = 5;
%p = a(10,40,50).foo;
%p = 5;
%%h = a.foo;
%%p = a(:,:,5).foo;