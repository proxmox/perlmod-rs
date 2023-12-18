#!/usr/bin/env perl

# Create a perl package given a product and package name.

use strict;
use warnings;

use File::Path qw(make_path);

my @packages;

my $opts = {
    'lib-tag' => [
        'TAG',
        'An identifier used to avoid loading multiple libraries with the same shared code',
    ],
    'lib-package' => [
        'Package',
        'Main package to generate for loading the library',
    ],
    'lib-prefix' => [
        'Prefix',
        'Package prefix used for documentation in the library package.',
    ],
    'lib' => [
        'LIBNAME',
        "The .so name without the 'lib' prefix.",
    ],
    'debug-libpath' => [
        'PATH',
        "Path to a debug library, usually ./target/debug.",
    ],
};

sub help {
    my ($fd) = @_;

    print {$fd} "usage: $0 OPTIONS <packages...>\n";
    print {$fd} "mandatory OPTIONS are:\n";
    for my $o (sort keys %$opts) {
        my ($arg, $desc) = $opts->{$o}->@*;
        my $p = "--$o=$arg";
        printf {$fd} "  %20s   %s\n", $p, $desc;
    }
}

if (!@ARGV) {
    help(\*STDERR);
    exit(1);
}

my $params = {};
ARGPARSE: while (@ARGV) {
    my $arg = shift @ARGV;

    last if $arg eq '--';

    if ($arg eq '-h' || $arg eq '--help') {
        help(\*STDOUT);
        exit(0);
    }

    for my $o (keys %$opts) {
        if ($arg =~ /^(?:--\Q$o\E=)(.+)$/) {
            my $arg = $1;
            die "multiple --$o options provided\n" if defined($params->{$o});
            $params->{$o} = $arg;
            next ARGPARSE;
        } elsif ($arg =~ /^--\Q$o\E$/) {
            $arg = shift @ARGV;
            die "multiple --$o options provided\n" if defined($params->{$o});
            die "--$o requires an argument\n" if !defined($arg);
            $params->{$o} = $arg;
            next ARGPARSE;
        }
    }

    if ($arg =~ /^-/) {
        help(\*STDERR);
        exit(1);
    }

    unshift @ARGV, $arg;
    last;
}

my $lib_package = $params->{'lib-package'}
    or die "missing --lib-package parameter\n";
my $lib_prefix = $params->{'lib-prefix'}
    or die "missing --lib-prefix parameter\n";
my $lib = $params->{'lib'}
    or die "missing --lib parameter\n";
my $lib_tag = $params->{'lib-tag'};
my $debug_libpath = $params->{'debug-libpath'} // '';

sub pkg2file {
    return ($_[0] =~ s@::@/@gr) . ".pm";
}

sub parentdir {
    if ($_[0] =~ m@^(.*)/[^/]+@) {
        return $1
    } else {
        die "bad path: '$_[0]', try adding a directory\n";
    }
}

my $template = <<'EOF';
package {{LIBRARY_PACKAGE}};

=head1 NAME

{{LIBRARY_PACKAGE}} - base module for {{LIBRARY_PREFIX}} rust bindings

=head1 SYNOPSIS

    package {{LIBRARY_PREFIX}}::RS::SomeBindings;

    use base '{{LIBRARY_PACKAGE}}';

    BEGIN { __PACKAGE__->bootstrap(); }

    1;

=head1 DESCRIPTION

This is the base module of all {{LIBRARY_PREFIX}} bindings.
Its job is to ensure the 'lib{{LIBRARY}}.so' library is loaded and provide a 'bootstrap'
method to load the actual code.

=cut

use strict;
use warnings;

use DynaLoader;

sub library {
    return '{{LIBRARY}}';
}

# Keep on a single line, modified by testsuite!
sub libdirs { return (map "-L$_/auto", @INC); }

sub load : prototype($) {
    my ($pkg) = @_;

    my $mod_name = $pkg->library();

    my @dirs = $pkg->libdirs();
    my $mod_file = DynaLoader::dl_findfile({{DEBUG_LIBPATH}}@dirs, $mod_name);
    die "failed to locate shared library for $mod_name (lib${mod_name}.so)\n" if !$mod_file;

    my $lib = DynaLoader::dl_load_file($mod_file)
	or die "failed to load library '$mod_file'\n";

    my $data = ($::{'{{LIBRARY_TAG}}-rs-library'} //= {});
    $data->{$mod_name} = $lib;
    $data->{-current} //= $lib;
    $data->{-package} //= $pkg;
}

sub bootstrap {
    my ($pkg) = @_;

    my $mod_name = $pkg->library();

    my $bootstrap_name = 'boot_' . ($pkg =~ s/::/__/gr);

    my $lib = $::{'{{LIBRARY_TAG}}-rs-library'}
	or die "rust library not available for '{{LIBRARY_PREFIX}}'\n";
    $lib = $lib->{$mod_name};

    my $sym  = DynaLoader::dl_find_symbol($lib, $bootstrap_name);
    die "failed to locate '$bootstrap_name'\n" if !defined $sym;
    my $boot = DynaLoader::dl_install_xsub($bootstrap_name, $sym, "src/FIXME.rs");
    $boot->();
}

BEGIN {
    __PACKAGE__->load();
    __PACKAGE__->bootstrap();
    init() if __PACKAGE__->can("init");
}

1;
EOF
$template =~ s/\{\{LIBRARY_PACKAGE\}\}/$lib_package/g;
$template =~ s/\{\{LIBRARY_PREFIX\}\}/$lib_prefix/g;
$template =~ s/\{\{LIBRARY_TAG\}\}/$lib_tag/g;
$template =~ s/\{\{LIBRARY\}\}/$lib/g;
$template =~ s/\{\{DEBUG_LIBPATH\}\}/$debug_libpath/g;

if ($lib ne '-') {
    my $path = pkg2file($lib_package);
    print "Generating $path\n";

    make_path(parentdir($path), { mode => 0755 });
    open(my $fh, '>', $path) or die "failed to open '$path' for writing: $!\n";
    print {$fh} $template;
    close($fh);
}

for my $package (@ARGV) {
    my $path = ($package =~ s@::@/@gr) . ".pm";

    print "Generating $path\n";

    $path =~ m@^(.*)/[^/]+@;
    make_path($1, { mode => 0755 });

    open(my $fh, '>', $path) or die "failed to open '$path' for writing: $!\n";
    print {$fh} "package $package;\n";
    print {$fh} "use base '$lib_package';\n";
    print {$fh} "BEGIN { __PACKAGE__->bootstrap(); }\n";
    print {$fh} "1;\n";
    close($fh);
}
