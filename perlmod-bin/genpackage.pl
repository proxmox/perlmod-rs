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
    'include-file' => [
        'PATH',
        "Path to additional perl code to include in the package after the 'use' statements",
    ],
    'from-notes' => [
        undef,
        "Read the package list from ELF notes sections",
    ],
};

sub help {
    my ($fd) = @_;

    print {$fd} "usage: $0 OPTIONS <packages...>\n";
    print {$fd} "mandatory OPTIONS are:\n";
    for my $o (sort keys %$opts) {
        my ($arg, $desc) = $opts->{$o}->@*;
        my $p = defined($arg) ? "--$o=$arg" : "--$o";
        printf {$fd} "  %20s   %s\n", $p, $desc;
    }
}

if (!@ARGV) {
    help(\*STDERR);
    exit(1);
}

my $params = {
    'include-file' => [],
};
ARGPARSE: while (@ARGV) {
    my $arg = shift @ARGV;

    last if $arg eq '--';

    if ($arg eq '-h' || $arg eq '--help') {
        help(\*STDOUT);
        exit(0);
    }

    for my $o (keys %$opts) {
        if ($arg =~ /^(?:--\Q$o\E=)(.+)$/) {
            $arg = $1;
        } elsif ($arg =~ /^--\Q$o\E$/) {
            $arg = shift @ARGV;
        } else {
            next;
        };

        if (!defined($opts->{$o}->[0])) {
            unshift @ARGV, $arg;
            $params->{$o} = 1;
            next ARGPARSE;
        }

        die "--$o requires an argument\n" if !defined($arg);
        if (ref($params->{$o}) eq 'ARRAY') {
            push $params->{$o}->@*, $arg;
        } else {
            die "multiple --$o options provided\n" if defined($params->{$o});
            $params->{$o} = $arg;
        }
        next ARGPARSE;
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
my $lib_tag = $params->{'lib-tag'}
    or die "missing --lib-tag parameter\n";
my $debug_libpath = $params->{'debug-libpath'} // '';
my $extra_code = '';
for my $file ($params->{'include-file'}->@*) {
    open(my $fh, '<', $file) or die "failed to open file '$file' - $!\n";
    my $more = do { local $/ = undef; <$fh> };
    die "error reading '$file': $!\n" if !defined($more);
    $extra_code .= $more;
}
my $from_notes = $params->{'from-notes'};

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

{{EXTRA_CODE}}
sub library { '{{LIBRARY}}' }

sub autodirs { map { "$_/auto" } @INC; }
sub envdirs { grep { length($_) } split(/:+/, $ENV{LD_LIBRARY_PATH} // '') }

sub find_lib {
    my ($mod_name) = @_;
    my @dirs = map { "-L$_" } (envdirs(), autodirs());
    return DynaLoader::dl_findfile(@dirs, $mod_name);
}

# Keep on a single line, potentially modified by testsuite!
sub libfile { find_lib(library()) }

sub load : prototype($) {
    my ($pkg) = @_;

    my $mod_name = $pkg->library();

    my $mod_file = libfile();
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
$template =~ s/\{\{EXTRA_CODE\}\}/$extra_code/g;
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

if ($from_notes) {
    die "missing library file to read packages from\n" if !@ARGV;
    die "--from-notes requires exactly one library\n" if @ARGV > 1;

    open my $fh, '<', $ARGV[0] or die "failed to open $ARGV[0] for reading: $!\n";
    my $fd = fileno($fh);
    my $data = `objcopy -O binary --only-section .note.perlmod.package $ARGV[0] /dev/stdout`;
    close($fh);

    my @packages;
    while (length($data)) {
        my ($name_size, $desc_size, $ty) = unpack('LLL', substr($data, 0, 3*4, ''));
        die "unexpected description in package note - incompatible perlmod version?\n"
            if $desc_size;
        my $name = substr($data, 0, $name_size, '');
        my $desc = substr($data, 0, $desc_size, '');
        print("Found package '$name'\n");
        push @packages, $name;
    }
    die "trailing data in notes section\n" if length($data);
    @ARGV = @packages;
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
