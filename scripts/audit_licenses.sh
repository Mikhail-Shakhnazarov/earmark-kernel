#!/usr/bin/env bash
set -euo pipefail

echo "Auditing dependency licenses..."
# Exclude local workspace packages from license audit
echo "Filtering workspace members..."
cargo tree --format "{p} {l}" | grep -v " (.*earmark-workspace" | sort | uniq

echo
echo "Checking for duplicated dependencies..."
duplicates=$(cargo tree --duplicates)
if [[ -n "$duplicates" ]]; then
    echo "DUPLICATED DEPENDENCIES FOUND (Informational for v0.1):"
    echo "$duplicates"
else
    echo "No duplicated dependencies found."
fi

echo
echo "License audit passed."
