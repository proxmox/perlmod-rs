#!/bin/sh

set -e

export CARGO=/usr/bin/cargo
export RUSTC=/usr/bin/rustc

CRATE=$1
BUILDCMD=${BUILDCMD:-"dpkg-buildpackage -b -uc -us"}

mkdir -p build
echo system >build/rust-toolchain
rm -rf "build/${CRATE}"

rm -f "${CRATE}/debian/control"
debcargo package --config "$PWD/${CRATE}/debian/debcargo.toml" --changelog-ready --no-overlay-write-back --directory "$PWD/build/${CRATE}" "${CRATE}" "$(dpkg-parsechangelog -l "${CRATE}/debian/changelog" -SVersion | sed -e 's/-.*//')"
cp "build/${CRATE}/debian/control" "${CRATE}/debian/control"
cd "build/${CRATE}"
${BUILDCMD}
