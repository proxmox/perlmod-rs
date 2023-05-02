# Shortcut for common operations:

CRATES=perlmod perlmod-macro

# By default we just run checks:
.PHONY: all
all: check

.PHONY: deb
deb: $(foreach c,$(CRATES), $c-deb)
	echo $(foreach c,$(CRATES), $c-deb)
	lintian build/*.deb

.PHONY: dinstall
dinstall:
	$(MAKE) clean
	$(MAKE) deb
	sudo -k dpkg -i build/librust-*.deb

%-deb:
	./build.sh $*
	touch $@

builddeps: $(foreach c,$(CRATES), $c-builddeps)
%-builddeps:
	BUILDCMD="mk-build-deps" ./build.sh $*

.PHONY: check
check:
	cargo test
	cargo build
	perl test.pl >out.test
	if diff -up out.test test.pl.expected; then rm out.test; \
	else echo "Test output mismatch between out.test and test.pl.expected"; fi

# Prints a diff between the current code and the one rustfmt would produce
.PHONY: fmt
fmt:
	cargo +nightly fmt -- --check

# Doc without dependencies
.PHONY: doc
doc:
	cargo doc --no-deps

.PHONY: clean
clean:
	cargo clean
	rm -rf build *-deb

.PHONY: update
update:
	cargo update

%-upload: %-deb
	cd build; \
	    dcmd --deb rust-$*_*.changes \
	    | grep -v '.changes$$' \
	    | tar -cf- -T- \
	    | ssh -X repoman@repo.proxmox.com upload --product devel --dist bullseye
