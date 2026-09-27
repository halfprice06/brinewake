#!/bin/sh
set -eu
cd "$(dirname "$0")"
exec cargo run --release -p bw_desktop --bin brinewake -- "$@"
