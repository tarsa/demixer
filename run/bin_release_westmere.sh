#!/bin/sh

RUSTFLAGS="-C target-cpu=westmere" cargo run --release --bin $@
