#!/bin/sh

RUSTFLAGS="-C target-cpu=haswell" cargo run --release --bin $@
