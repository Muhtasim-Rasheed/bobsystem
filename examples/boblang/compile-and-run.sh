#!/usr/bin/env bash

file_bobl=$1
file_bobj="${file_bobl%.*}.bobj"
file_bobin="${file_bobl%.*}.bobin"
rt_basm="$(dirname -- $file_bobl)/rt.basm"
rt_bobj="$(dirname -- $file_bobl)/rt.bobj"

cargo run --bin basm -- $rt_basm $rt_bobj &&
cargo run --bin boblc -- $file_bobl $file_bobj &&
cargo run --bin bl -- $file_bobj $rt_bobj -o $file_bobin &&
time cargo run --release --bin bob16 -- $file_bobin

rm $file_bobj $file_bobin $rt_bobj
