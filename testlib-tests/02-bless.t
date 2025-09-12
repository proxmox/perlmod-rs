use v5.36;

use Test::More tests => 4;

use TestLib::BlessBox;

my $guard = {};
my $inner = { content => 'some test' };
my $obj = TestLib::BlessBox->new($inner->{content}, $guard);
is(ref($obj), 'TestLib::BlessBox', 'object was blessed into package');
is_deeply($obj->raw_method(), $inner, 'raw_method returns the right data');
is_deeply($obj->method(), $inner, 'method returns the right data');
undef $obj;
is($guard->{destroyed}, 44, 'object destructor ran');
