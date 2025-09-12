use v5.36;

use Test::More tests => 8;

use bigint 'hex'; # perl you sh...

use TestLib::Digest;
use TestLib::Hello;

my $str = "Hello You";
is(TestLib::Digest::fnv64a($str), hex('0xdb61ca777f4b8ba0'), "'Hello You' fnv64a hash wrong");
is(TestLib::Digest::fnv64a(substr($str, 3, 3)), hex('0x1250b4191dafc2a4'), "'lo ' fnv64a hash wrong");

is(TestLib::Hello::opt_string(substr($str, 3, 3)), "Called with \"lo \".", "substr passed to Option<String>");
is(TestLib::Hello::opt_string(undef), "Called with None.", "undef passed to Option<String>");
is(TestLib::Hello::opt_str(substr($str, 3, 3)), "Called with \"lo \".", "substr passed to Option<&str>");
is(TestLib::Hello::opt_str(undef), "Called with None.", "undef passed to Option<&str>");

is(TestLib::Digest::fnv64a("emoji 🤖"), hex('0x5631099f8622bde8'), "emoji :robot: fnv64a hash");
{
	use utf8;
	is(TestLib::Digest::fnv64a("emoji 🤖"), hex('0x5631099f8622bde8'), "emoji :robot: fnv64a hash with 'use utf-8'");
}
