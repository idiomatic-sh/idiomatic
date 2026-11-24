# Repository query benchmark

The reference workload is the pinned Tastemaker release running `tm check --format sarif` in each CI workspace. The coordinator's `tm-bench/scripts` captures SDK calls and their answers in paired JSON lines. The installed binary, workload checkout, CPU and memory measurement protocol and SARIF hashes are recorded with that capture.

Build the Ryo CLI's `vec-readers` Cargo bench. From this directory, run the scope controls and replay a captured check with the resulting executable.

```sh
python3 check_scope.py "$VEC_READERS" "$SHIPPING_READERS"
python3 replay.py original-ryo.trace.gz "$VEC_READERS" project-ryo.tsv
```

`check_scope.py` compares positive and empty results against the shipping SDK. It also exercises missing and partial populations. `replay.py` covers every captured invocation, including those with a partial reference. It passes the captured query text, parameters, complete inventory, rule admission and typed host rows into one process. It reverses file visitation and table rows on the second evaluation. The TSV records complete typed equality, a mismatch, an order change, a refusal or a partial reference separately for each rule invocation. A receipt hashes sorted rows with all columns, the subject and the ordered key. Expected rows never enter evaluation. An optional fourth argument writes the forward-only input stream for timing a fresh executor process.

This benchmark exercises consumers of captured data. A whole-check gate also needs live placement, observation producers, suppressions and a byte comparison of the resulting SARIF.

## Live check on the measured release

The live benchmark replaces the typed lint call in Tastemaker 0.1.3 with the project session. The release still constructs the input from placement, templates and settings. It feeds each answer into later checks, applies suppressions and renders SARIF. The release's own linked readers supply syntax, semantic facts and project resolution.

Create a fresh archive of the release and link it to the adapter in this checkout. The linker writes only into the supplied archive. Set `CARGO_TARGET_DIR` to the worktree's target directory and run the build inside the assigned build lane.

```sh
release=$(mktemp -d)
git archive 1c7f3f7db86e45139b6eb4005508f3c2d4040860 crates | tar -x -C "$release"
python3 crates/tastemaker/benchmarks/project/live/link-release.py "$release"
(cd "$release/crates/tastemaker" && CARGO_PROFILE_BENCH_DEBUG=line-tables-only cargo build --offline --profile bench --bin tm --target-dir "$CARGO_TARGET_DIR")
```

Run the resulting `release/tm check --format sarif` and the pinned executable at the same absolute workspace path with a fresh product store for each. The benchmark owner's `whole.execute` runs the normal command, measures total process CPU, elapsed wall time and maximum RSS, and removes the generated store. The same checkout path makes raw byte comparison possible because SARIF contains its URI. A pinned single-core run selects one worker. The benchmark writes the number of live sessions, project population builds and phase clocks to stderr. The raw report stays on stdout.
