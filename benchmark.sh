#!/usr/bin/env bash

cargo build --release
hyperfine --warmup 2 -m 3 target/release/breezy
