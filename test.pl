#!/usr/bin/env perl

use v5.28.0;

use lib '.';
use RSPM::Bless;
use RSPM::Foo142;
use RSPM::Option;

my $v = RSPM::Bless->new("Hello");
$v->something();
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
