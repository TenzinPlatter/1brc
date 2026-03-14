#!/usr/bin/env bash

set -xeo pipefail

cargo build --release
hyperfine --warmup 3 -m 3 target/release/breezy
