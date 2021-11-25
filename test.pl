#!/usr/bin/env perl

use v5.28.0;

use POSIX ();

# The nasty ones:
use Storable;
use Clone;

use lib '.';
use RSPM::Bless;
use RSPM::Foo142;
use RSPM::Option;
use RSPM::Magic;

STDOUT->autoflush;
# Let's combine stderr and stdout:
POSIX::dup2(fileno(STDOUT), fileno(STDERR));

my $v = RSPM::Bless->new("Hello");
$v->something();
$v->something_nonraw();
my ($a, $b, $c) = $v->multi_return();
say "Got ($a, $b, $c)";
my @ret = $v->multi_return();
say "Got: ".scalar(@ret)." values: @ret";

$v->another(54);

my $param = { a => 1 };
my $s = "Hello You";
print "These should be called with a valid substr:\n";
RSPM::Foo142::test(substr($s, 3, 3));
RSPM::Foo142::teststr(substr($s, 3, 3));
print "Parameter exists: " . (exists($param->{x}) ? "YES" : "NO") . "\n";
RSPM::Foo142::test($param->{x});
print "Was auto-vivified: " . (exists($param->{x}) ? "YES" : "NO") . "\n";
RSPM::Foo142::teststr($param->{x});

my $a = "Can I have some coffee please?\n";
print $a;
my $b = RSPM::Foo142::test_serde($a);
print $b;
my $c = RSPM::Foo142::test_serde($b);
print $c;

use utf8;
binmode STDOUT, ':utf8';
my $a = "Can I have some ☕ please?\n";
print $a;
my $b = RSPM::Foo142::test_serde($a);
print $b;
my $c = RSPM::Foo142::test_serde($b);
print $c;

sub to_string {
    my ($param) = @_;

    my $state = $param->{tristate};
    $state = int($state) if defined($state);

    my $a;
    if (defined($state)) {
	$a = $state ? "Some(true)" : "Some(false)";
    } else {
	$a = "None";
    }

    my $b = RSPM::Option::to_string($state);
    my $c = RSPM::Option::struct_to_string({ 'tristate' => $state });

    print "$a\n";
    print "$b\n";
    print "$c\n";
}

to_string({ 'tristate' => '0' });
to_string({ 'tristate' => '1' });
to_string({ 'tristate' => undef });

my $ref1 = { x => "x was stored" };
my $ref2 = RSPM::Foo142::test_refs({ copied => "copied string", reference => $ref1 });
print($ref1->{x}, "\n");
$ref2->{x} = "x was changed";
print($ref1->{x}, "\n");

my $magic = RSPM::Magic->new('magic test');
$magic->call();

sub test_unsafe_clone($) {
    my ($bad) = @_;
    eval { $bad->call() };
    if (!$@) {
        die "cloned object not properly detected!\n";
    } elsif ($@ ne "value blessed into RSPM::Magic did not contain its declared magic pointer\n") {
        die "cloned object error message changed to: [$@]\n";
    }
    undef $bad;
    print("unsafe dclone dropped\n");
}

print("Testing unsafe dclone\n");
test_unsafe_clone(Storable::dclone($magic));

print("Testing unsafe clone\n");
test_unsafe_clone(Clone::clone($magic));
undef $magic;

print("Testing enum deserialization\n");
my $ra = RSPM::Foo142::test_enums("something");
die "unexpected result from test_enums: $ra\n" if $ra ne 'result-a';

print("Testing optional parameters\n");
RSPM::Foo142::test_trailing_optional(1, 99);
RSPM::Foo142::test_trailing_optional(2, undef);
RSPM::Foo142::test_trailing_optional(3);
