#!/bin/bash
#:
#: name = "build"
#: variety = "basic"
#: target = "helios"
#: rust_toolchain = "stable"
#: output_rules = [
#:   "/work/debug/*",
#:   "/work/release/*",
#: ]
#:

set -o errexit
set -o pipefail
set -o xtrace

cargo --version
rustc --version

banner "build"
ptime -m cargo build
ptime -m cargo build --release

banner "check"
cargo fmt -- --check
cargo clippy

for x in debug release
do
    mkdir -p /work/$x
    cp target/$x/p9kp /work/$x/p9kp
done
