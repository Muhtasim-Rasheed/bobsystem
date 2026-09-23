#!/usr/bin/env bash

file_basm=$1
file_bobj="${file_bobl%.*}.bobj"
file_bobin="${file_bobl%.*}.bobin"

cargo run --bin basm -- $file_basm $file_bobj
cargo run --bin bl -- $file_bobj -o $file_bobin

cargo run --release --bin bob16 -- $file_bobin
