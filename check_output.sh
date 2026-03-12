#!/usr/bin/env bash

cargo build --release
diff data/answers.txt <(./target/release/breezy)
