#!/usr/bin/env bash

set -xeo pipefail

cargo build --release
hyperfine --warmup 2 -m 3 target/release/breezy
