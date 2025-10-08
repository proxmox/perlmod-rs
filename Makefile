# Shortcut for common operations:

CRATES=perlmod perlmod-macro

# By default we just run checks:
.PHONY: all
all: check

.PHONY: deb
deb: $(foreach c,$(CRATES), $c-deb)
	echo $(foreach c,$(CRATES), $c-deb)
	lintian build/*.deb

.PHONY: dsc
dsc: $(foreach c,$(CRATES), $c-dsc)
	echo $(foreach c,$(CRATES), $c-dsc)
	lintian build/*.dsc

.PHONY: dinstall
dinstall:
	$(MAKE) clean
	$(MAKE) deb
	sudo -k dpkg -i build/librust-*.deb

perlmod-bin-deb:
	mkdir build || true
	rm -rf build/perlmod-bin-deb
	git archive --format=tar HEAD perlmod-bin | tar -C build -xf -
	cd build/perlmod-bin && dpkg-buildpackage --no-sign -b

%-deb:
	./build.sh $*
	touch $@

%-dsc:
	BUILDCMD='dpkg-buildpackage -S -us -uc -d' NOTEST=1 ./build.sh $*
	touch $@

%-autopkgtest:
	autopkgtest build/$* build/*.deb -- null
	touch $@

builddeps: $(foreach c,$(CRATES), $c-builddeps)
%-builddeps:
	BUILDCMD="mk-build-deps" ./build.sh $*

.PHONY: check
check:
	cargo test

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
	rm -rf build *-deb *-dsc

.PHONY: update
update:
	cargo update

%-upload: %-deb
	cd build; \
	    dcmd --deb ./*$*_*.changes \
	    | grep -v '.changes$$' \
	    | tar -cf- -T- \
	    | ssh -X repoman@repo.proxmox.com upload --product devel --dist bookworm
