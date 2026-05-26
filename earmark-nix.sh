#!/usr/bin/env bash
args=$(printf " %q" "$@")
nix develop --command bash -lc "cargo run --quiet --bin earmark-cli -- $args"
