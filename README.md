perlmod
=======

This is a rust crate which allows exporting rust modules as perl packages.

The initial use case for this was to help migrating a perl codebase to rust.

This crate can be compared to perl xs, however, it does not expose the complete power of perl, only
enough to make callable methods. The biggest part is the serde serializer and deserializer,
providing ways to transfer data between perl and rust.

State of this crate
===================

This crate is functional and supports a subset perl that the authors consider "sane enough" to work
with. It may misbehave when faced with dark perl magic (technical term), but should be enough for a
lot of use cases.

This crate is being used by Proxmox to implement parts of Proxmox VE and Proxmox Mail Gateway, and
is being maintained as part of these products.

Change Logs
===========

Since we maintain debian build scripts, see the `debian/changelog` files in `proxmox/` and
`proxmox-macro/` for their respective changes.

Where to report bugs
====================

https://bugzilla.proxmox.com

Licensing, linking to libperl and adding more features
======================================================

The perl license explicitly states that code linking to the perl library for the purpose of merely
providing subroutines or variables is considered part of its "input", provided it does not change
the language in any way that would cause it to fail its regression tests. It does not consider its
"input" to fall under perl's copyright.

In order to avoid confusion about licensing or copyright, this crate does not aim to provide
complete interoperability, but is meant merely as an alternative to using "xs" for writing bindings
to rust code.

Features which would allow changing this (other than by obviously unintended behavior (such as
bugs, raw memory access, etc.)) will not be accepted into this crate and will need to be maintained
elsewhere.
