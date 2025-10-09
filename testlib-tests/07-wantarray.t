use v5.36;

use Test::More tests => 8;

use TestLib::Ret;

my @many = TestLib::Ret::get_tuple();
is_deeply(\@many, [qw(first second)], "get_tuple() should return 2 values");

@many = TestLib::Ret::maybe_many();
is_deeply(\@many, [qw(multiple values)], "maybe_many() in list context returns 2 values");
my $one = TestLib::Ret::maybe_many();
is($one, 'single value', "maybe_many() in scalar context should return 1 value");

@many = TestLib::Ret::try_maybe_many();
is_deeply(\@many, [qw(try multiple values)], "try_maybe_many() in list context returns 3 values");
@many = eval { TestLib::Ret::try_maybe_many('errL') };
is(
    $@,
    "failed in list context (errL)\n",
    'try_maybe_many("errL") in list context fails with correct error message',
);

$one = TestLib::Ret::try_maybe_many();
is($one, 'try single value', "try_maybe_many() in scalar context should return 1 value");
$one = eval { TestLib::Ret::try_maybe_many('errS') };
is(
    $@,
    "failed in scalar context (errS)\n",
    'try_maybe_many("errS") in scalar context fails with correct error message',
);

eval { TestLib::Ret::try_maybe_many("VOID") };
is(
    $@,
    "failed in void context (VOID)\n",
    'try_maybe_many("VOID") in void context fails with correct error message',
);
