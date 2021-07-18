#!/bin/sh

cargo build --release
yes | cp ./target/release/nirust .
./nirust
