use v5.36;

use Storable ();
use Clone ();

use Test::More tests => 7;

use TestLib::Magic;

ok(!TestLib::Magic::is_dropped(), "instance was not dropped yet");
my $obj = TestLib::Magic->new("The Content");
is($obj->call(), 'magic box content "The Content"', 'method call works');

my sub test_bad_clone ($bad) {
    my $ret = eval { $bad->call() };
    is($@, "value blessed into TestLib::Magic did not contain its declared magic pointer\n",
        "dclone() object should fail");
    ok(!$ret, "dclone() method call should not return anything");
}
test_bad_clone(Storable::dclone($obj));
test_bad_clone(Clone::clone($obj));

undef $obj;
ok(TestLib::Magic::is_dropped(), "instance was dropped");
