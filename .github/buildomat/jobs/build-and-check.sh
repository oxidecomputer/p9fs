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
#: [[publish]]
#: series = "p9kp"
#: name = "p9kp"
#: from_output = "/work/release/p9kp"
#:
#: [[publish]]
#: series = "p9kp"
#: name = "p9kp.sha256"
#: from_output = "/work/release/p9kp.sha256"

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
    sha256sum /work/$x/p9kp > /work/$x/p9kp.sha256
done
