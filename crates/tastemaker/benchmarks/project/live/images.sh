#!/bin/bash
# usage: images.sh REV OUTDIR [PROGRAM...]      PROGRAM: tm vec vec_ts vec_rust (default: all four; `images.sh REV OUTDIR vec` builds only vec)
# The benchmark binaries from the workspace's own sources at REV (no released host, no link step): `tm` (the Tastemaker bench package, this folder)
# and `vec`, `vec_ts`, `vec_rust` (the linter images, crates/ryo/crates/executor/benches/lint_image). Run in a build lane.
# Writes OUTDIR/<each program built>, OUTDIR/revision and OUTDIR/SHA256SUMS (the programs built). The two packages are workspaces of their own: each builds
# with its product's lock file, copied beside it first (not committed), in the bench profile (fat LTO, one codegen unit). Only the programs asked for are compiled.
# Environment (all optional): IMAGES_TARGET (cargo target folder, default OUTDIR/target; a third argument that contains a slash is read as this folder, the old form),
# IMAGES_CORES (taskset, default 4-9), CARGO_BUILD_JOBS (default 4), IMAGES_WAIT_SCRIPT (a script that returns when no timing window is open in the INBOX; the
# gate's default is used when it exists).
set -euo pipefail
repo=$(git rev-parse --show-toplevel)
rev=$(git -C "$repo" rev-parse "$1^{commit}")
out=$2; shift 2
target=${IMAGES_TARGET:-$out/target}
if [[ ${1:-} == */* ]]; then target=$1; shift; fi
programs=("$@"); [[ ${#programs[@]} -gt 0 ]] || programs=(tm vec vec_ts vec_rust)
for p in "${programs[@]}"; do case $p in tm|vec|vec_ts|vec_rust) ;; *) echo "images.sh: unknown program '$p' (tm vec vec_ts vec_rust)" >&2; exit 2;; esac; done
wait_script=${IMAGES_WAIT_SCRIPT:-/workspaces/ryo-poc-evidence/next/build/ports/wait-timing-window.sh}
[[ -x $wait_script ]] && "$wait_script" images.sh
src=$out/src; mkdir -p "$src" "$target"
# `.cargo` comes with the archive: its `[env]` builds jemalloc's options into every binary (narenas:4, dirty_decay_ms:300, muzzy_decay_ms:0) and it links with mold:
# without it the images peak about 450 MB higher at ten workers than the product (measured) and spend about 5% less CPU
git -C "$repo" archive "$rev" .cargo crates rust-toolchain.toml | tar -xf - -C "$src"
live=$src/crates/tastemaker/benchmarks/project/live
vec=$src/crates/ryo/crates/executor/benches/lint_image
cp "$src/crates/tastemaker/Cargo.lock" "$live/Cargo.lock"
cp "$src/crates/ryo/Cargo.lock" "$vec/Cargo.lock"
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4} CARGO_PROFILE_BENCH_DEBUG=line-tables-only
cores=${IMAGES_CORES:-4-9}
cd "$src"
tm_bins=(); vec_bins=()
for p in "${programs[@]}"; do if [[ $p == tm ]]; then tm_bins+=(--bin tm); else vec_bins+=(--bin "$p"); fi; done
if [[ ${#tm_bins[@]} -gt 0 ]]; then
  taskset -c "$cores" cargo build --offline --profile bench --manifest-path "$live/Cargo.toml" "${tm_bins[@]}" --target-dir "$target/tm" --message-format short
fi
if [[ ${#vec_bins[@]} -gt 0 ]]; then
  taskset -c "$cores" cargo build --offline --profile bench --manifest-path "$vec/Cargo.toml" "${vec_bins[@]}" --target-dir "$target/vec" --message-format short
fi
for p in "${programs[@]}"; do
  if [[ $p == tm ]]; then cp --reflink=auto "$target/tm/release/tm" "$out/tm"; else cp --reflink=auto "$target/vec/release/$p" "$out/$p"; fi
done
echo "$rev" > "$out/revision"
(cd "$out" && sha256sum "${programs[@]}" > SHA256SUMS)
echo IMAGES-DONE
