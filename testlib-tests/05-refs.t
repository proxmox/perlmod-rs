use v5.36;

use Test::More tests => 7;

use TestLib::Refs;

my $ref1 = { x => 'initialized' };
my $returned_ref = TestLib::Refs::get_ref_from_test_struct({ text => "Some Text", reference => $ref1 });
ok(!!$returned_ref, 'returned_ref is not undef');
is(ref($returned_ref), 'HASH', "a hash reference is returned");
is($returned_ref->{x}, 'initialized', 'looks like the correct hash');
$returned_ref->{x} = 'x was changed';
is($ref1->{x}, 'x was changed', 'original hash was referenced');

my $str = 'OneTwoThree';
my $sub = TestLib::Refs::get_substr($str);
is($str, 'OneTwoThree', 'original string should be unmodified');
is($sub, 'Two', 'test substr() return');
$sub = 999;
is($str, 'OneTwoThree', 'original string should (still) be unmodified');
