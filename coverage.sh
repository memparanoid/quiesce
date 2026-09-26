#!/usr/bin/env sh
# Generates the HTML coverage report and opens it in the browser.
# Report lands in target/llvm-cov/html (already git-ignored via /target).
set -eu

cargo llvm-cov nextest --html --open "$@"
