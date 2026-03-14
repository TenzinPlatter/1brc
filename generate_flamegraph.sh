cargo build --release
RUSTFLAGS="-C force-frame-pointers=yes -C target-cpu=native" cargo flamegraph
zen-browser ./flamegraph.svg
