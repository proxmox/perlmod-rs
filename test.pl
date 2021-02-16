#!/usr/bin/env perl

use v5.28.0;

use lib '.';
use RSPM::Bless;

my $v = RSPM::Bless->new("Hello");
$v->something();
my ($a, $b, $c) = $v->multi_return();
say "Got ($a, $b, $c)";
my @ret = $v->multi_return();
say "Got: ".scalar(@ret)." values: @ret";

$v->another(54);
