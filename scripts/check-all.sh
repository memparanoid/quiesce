#!/usr/bin/env bash
# Copyright (c) 2026 Federico Hoerth <memparanoid@gmail.com>
# SPDX-License-Identifier: Apache-2.0
# See LICENSE in the repository root for full license text.
#
# Formatting once, and clippy on every platform this crate claims to support.
#
# # Why targets and not containers
#
# Most of what differs between platforms is decided by `cfg` and settled at
# check time, so `--target` reaches every target in `ALL` without a machine or an
# image for any of them.
#
# What it does not reach is anything that only shows when the code runs. That is
# what the CI matrix is for; this is what catches the `cfg` that was never
# compiled before the push.
#
# # A missing target is a failure, not a skip
#
# A check-all that quietly checked some of them is worse than no check at all,
# because it reports success. Anything not installed is named, with the line
# that installs it, and the run ends red.
#
# # Arguments
#
# Target triples, to narrow it: `check-all.sh x86_64-pc-windows-msvc`.
# With none, every target in `ALL`.

set -uo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT"

# The CI matrix, plus the other architecture of each. Windows is `msvc` because
# that is what `windows-latest` is.
ALL=(
  x86_64-unknown-linux-gnu
  aarch64-unknown-linux-musl
  x86_64-apple-darwin
  aarch64-apple-darwin
  x86_64-pc-windows-msvc
  aarch64-pc-windows-msvc
)

WANTED=("$@")

[ ${#WANTED[@]} -eq 0 ] && WANTED=("${ALL[@]}")

INSTALLED="$(rustup target list --installed)"

PASSED=()
FAILED=()
MISSING=()

echo
echo "################################################################"
echo "## formatting"
echo "################################################################"
echo

if cargo fmt --all -- --check; then
  echo "  clean"
else
  echo
  echo "  FAIL — run \`cargo fmt --all\`"
  FAILED+=("fmt")
fi

for target in "${WANTED[@]}"; do
  echo
  echo "################################################################"
  echo "## $target"
  echo "################################################################"
  echo

  if ! grep -qx "$target" <<<"$INSTALLED"; then
    echo "  not installed: rustup target add $target"
    MISSING+=("$target")
    continue
  fi

  # Both ways round, like the lint before it: a lint that only ever sees one
  # set of features never sees the code behind the others.
  if cargo clippy --color=always --workspace --all-targets --target "$target" -- -D warnings \
    && cargo clippy --color=always --workspace --all-targets --all-features --target "$target" -- -D warnings; then
    PASSED+=("$target")
  else
    FAILED+=("$target")
  fi
done

echo
echo "################################################################"
echo "## summary"
echo "################################################################"
echo

for one in "${PASSED[@]}"; do
  echo "  pass     $one"
done

for one in "${FAILED[@]}"; do
  echo "  FAIL     $one"
done

for one in "${MISSING[@]}"; do
  echo "  MISSING  $one"
done

if [ ${#MISSING[@]} -gt 0 ]; then
  echo
  echo "  rustup target add ${MISSING[*]}"
fi

echo

[ ${#FAILED[@]} -eq 0 ] && [ ${#MISSING[@]} -eq 0 ]
