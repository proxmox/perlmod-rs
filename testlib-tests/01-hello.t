use v5.36;

use Test::More tests => 10;

use TestLib::Hello;

my $greeting = eval { TestLib::Hello::hello("Testsuite") } // $@;
is($greeting, "Hello, 'Testsuite'", 'test library loads correctly');

my ($x, $y) = TestLib::Hello::multi_return();
is($x, 17, "first multi_return value should be 17");
is($y, 32, "second multi_return value should be 32");

my $param = { a => 1 };
is(TestLib::Hello::opt_string($param->{x}), "Called with None.", "non-existent element passed to Option<String>");
ok(!exists($param->{x}), "param->{x} was not auto-vivified");
is(TestLib::Hello::opt_str($param->{x}), "Called with None.", "non-existent element passed to Option<&str>");
ok(!exists($param->{x}), "param->{x} was not auto-vivified (2)");

is(TestLib::Hello::trailing_optional(1, 99), '1, Some(99)', 'passing value for trailing optional parameter');
is(TestLib::Hello::trailing_optional(2, undef), '2, None', 'passing undef for trailing optional parameter');
is(TestLib::Hello::trailing_optional(3), '3, None', 'skipping trailing optional parameter');
