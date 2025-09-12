use v5.36;

use Test::More tests => 17;

use utf8;
binmode STDOUT, ':utf8';

use TestLib::Hello;
use TestLib::Digest;

is(TestLib::Hello::new_string_param("direct string"), "string contained \"direct string\"", "pass string to serde newtype string");
my $str = 'Hello You';
is(TestLib::Hello::new_string_param($str), "string contained \"Hello You\"", "pass string variable to serde newtype string");

is(TestLib::Hello::new_string_param("direct ☕"), "string contained \"direct ☕\"", "pass utf-8 string to serde newtype string");
my $coffee = "utf-8 coffee (☕) string";
is(TestLib::Hello::new_string_param($coffee), "string contained \"utf-8 coffee (☕) string\"", "pass utf-8 string variable to serde newtype string");

sub test_struct_opt_string ($value) {
    $value = int($value) if defined($value);

    my $a;
    if (defined($value)) {
	$a = $value ? "Some(true)" : "Some(false)";
    } else {
	$a = "None";
    }

    my $b = TestLib::Hello::opt_bool_to_string($value);
    my $c = TestLib::Hello::struct_opt_to_string({ value => $value });

    my $value_s = defined($value) ? $value : 'undef';
    is($a, $b, "opt_bool_to_string($value_s) should be $a");
    is($b, $c, "struct_opt_to_string($value_s) should be $a");
}
test_struct_opt_string(0);
test_struct_opt_string(1);
test_struct_opt_string(undef);

is(TestLib::Hello::map_an_enum("something"), "result-a", "AnEnum::Something should map to AnEnum::ResultA");
is(TestLib::Hello::map_an_enum("another"), "result-b", "AnEnum::Another should map to AnEnum::ResultB");
my $res = eval { TestLib::Hello::map_an_enum("result-a") };
is($@, "invalid\n", 'map_an_enum("result-a") should croak with "invalid\n"');
ok(!$res);

is_deeply(TestLib::Hello::map_tagged_enum({ 'in1' => 'a' }), { 'out1' => 'a.' }, "tagged enum in1 should map to out1");
$res = eval { TestLib::Hello::map_tagged_enum({ 'out1' => 'a' }) };
is($@, "out1\n", 'map_tagged_enum("out1") should croak with "out1\n"');
ok(!$res);
