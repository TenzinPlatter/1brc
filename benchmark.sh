#!/usr/bin/env bash

set -xeo pipefail

cargo build --release
hyperfine --warmup 5 -m 5 target/release/breezy
