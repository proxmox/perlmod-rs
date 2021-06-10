#!/usr/bin/env perl

use v5.28.0;

use lib '.';
use RSPM::Bless;
use RSPM::Foo142;

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
