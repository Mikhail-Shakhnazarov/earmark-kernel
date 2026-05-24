#!/usr/bin/env bash
args=$(printf " %q" "$@")
nix-shell --quiet -p pkg-config openssl --run "cargo run --quiet --bin earmark-cli -- $args"
