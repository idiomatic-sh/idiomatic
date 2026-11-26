#!/bin/bash
# usage: images.sh REV OUTDIR [TARGET_DIR]
# The five benchmark binaries from the workspace's own sources at REV (no released host, no link step): `tm` and `tm_drop` (the Tastemaker
# bench package, this folder) and `vec`, `vec_ts`, `vec_rust` (the linter images, crates/ryo/crates/engine/benches/lint_image). Run in a build lane.
# Writes OUTDIR/{tm,tm_drop,vec,vec_ts,vec_rust}, OUTDIR/revision and OUTDIR/SHA256SUMS. The two packages are workspaces of their own: each
# builds with its product's lock file, copied beside it first (not committed), in the bench profile (fat LTO, one codegen unit).
set -euo pipefail
repo=$(git rev-parse --show-toplevel)
rev=$(git -C "$repo" rev-parse "$1^{commit}")
out=$2; target=${3:-$out/target}
src=$out/src; mkdir -p "$src" "$target"
git -C "$repo" archive "$rev" crates rust-toolchain.toml | tar -xf - -C "$src"
live=$src/crates/tastemaker/benchmarks/project/live
vec=$src/crates/ryo/crates/engine/benches/lint_image
cp "$src/crates/tastemaker/Cargo.lock" "$live/Cargo.lock"
cp "$src/crates/ryo/Cargo.lock" "$vec/Cargo.lock"
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4} CARGO_PROFILE_BENCH_DEBUG=line-tables-only
cores=${IMAGES_CORES:-4-9}
cd "$src"
taskset -c "$cores" cargo build --offline --profile bench --manifest-path "$live/Cargo.toml" --bins --target-dir "$target/tm" --message-format short
taskset -c "$cores" cargo build --offline --profile bench --manifest-path "$vec/Cargo.toml" --bins --target-dir "$target/vec" --message-format short
for f in tm tm_drop; do cp --reflink=auto "$target/tm/release/$f" "$out/$f"; done
for f in vec vec_ts vec_rust; do cp --reflink=auto "$target/vec/release/$f" "$out/$f"; done
echo "$rev" > "$out/revision"
(cd "$out" && sha256sum tm tm_drop vec vec_ts vec_rust > SHA256SUMS)
echo IMAGES-DONE
