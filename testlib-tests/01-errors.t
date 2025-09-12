use v5.36;

use Test::More tests => 6;

use TestLib::Errors;

my $res = eval { TestLib::Errors::my_error(0) };
ok($!{EBADFD}, "my_error(false) should succeed and set errno to 77 (EBADFD)");
is($res, 'worked', 'my_error(false) should return "worked"');

$res = eval { TestLib::Errors::my_error(1) };
my $err = $@;
ok(ref($err), 'my_error(1) should fail with structured error');
is(ref($err), 'HASH', 'my_error(1) should fail with a HASH as error');
is_deeply($err, { a => "first", b => "second" }, 'my_error(1) should fail with specific hash');
ok(!$res);
