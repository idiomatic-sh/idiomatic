# Tastemaker check benchmark

Build the reference workers from the release tag named by the root `tastemaker.toml`, and the candidate from the current committed engine. Use a scratch directory outside the checkout. The commands below run from the repository root with Python 3.11 or later, Git, Cargo, the repository toolchain and the locked dependencies available to Cargo offline.

```sh
pack=crates/tastemaker/benchmarks/check
python3 "$pack/build.py" "$workers" --target-dir "$CARGO_TARGET_DIR"
python3 "$pack/bench.py" verify-reference-cases "$workers"
python3 "$pack/bench.py" verify-cases "$workers"
python3 "$pack/bench.py" measure "$workers" --tm "$pinned_tm" --json "$result"
```

Set `workers` to the output directory, `pinned_tm` to the released executable and `result` to a new report path. The measurement requires Linux for child CPU, affinity and peak RSS accounting. An existing CPU affinity is respected. Rebuild the candidate after changing the engine.

## Frozen cases

Each directory under `tests/fixtures/rules` has the exact query bytes and a JSON record per case. Each record identifies its selected rule, its original SDK call’s companion rules, parameters, admission, typed caller tables, Given files and beside overrides. The expected answer lists the full typed rows and unknowns. Input maps point to content-addressed files under `tests/fixtures/resources`. The checkout under test supplies no case input. The suffix on a query directory is the start of its SHA-256.

The frozen sample comes from commit `6418d3eb576e8ff670da86836f648a2c513ef57b`. It retains each of the 271 texts and supplementary cases for the observed dependency, outcome and empty or positive answer. The release and candidate each run all 294 cases. The candidate runs three fresh processes in each of the forward and reversed admission and table orders. The exit status is nonzero when any required answer differs, is refused, is partial or changes across orders. A completed empty answer must have a receipt. The case gate compares the typed row digest and the sorted multiset of unknown path/message pairs. The current native batch receipt has three tab-separated fields and carries no unknowns. A worker can append a fourth field containing its JSON array of path/message pairs. This lets the five cases with release unknowns pass when both parts of the answer match.

## Live corpus

`measure` checks out the current commit in temporary storage. It runs each workspace’s actual SARIF command with a fresh product store. A release observer captures the SDK calls, typed caller inputs and sorted typed answer digests. A separate pristine release process must reproduce the observer’s entire SARIF byte for byte. The reference is regenerated on every invocation. Every original request counts, including empty requests and unknowns.

The candidate runs in one process on one CPU. It runs the checkout’s registered `vec-readers --tm-batch` entry. The session receives each query, parameter set, source root, admission, complete numbered inventory and demanded caller tables. The expected digest field of its manifest is a fixed non-digest sentinel. The benchmark compares the emitted canonical digests outside the worker, after execution. This driver exercises captured host data. It does not produce a candidate SARIF. Preparing capture files and comparing receipts are outside the candidate timer. The release time covers the whole CI check, including reporting. Both times include process startup, query reading and writes. The primary result is user plus system CPU; wall seconds and peak MiB follow. The target on the Ryo workspace is five total CPU-seconds for the engine session with every request equal to the release. Live host discovery, store work, templates, suppression and SARIF belong to the separate release CI total. The part rows come from the process with the middle total CPU (lower middle for even repeat counts). The residual includes startup, scheduling, teardown and timer rounding. Newer workers split the reader row into version reads, parser and provider build, and projection and facts. The JSON retains the additive clocks for every repeat.

Three cold paired runs per workspace are the default. A changing answer is unstable. An SDK unknown is partial. Use `--workspace` to select a workload and `--repeats` to change the number of pairs. To reuse separately built executables, replace the workers directory on the measure command with `--reader PATH --observer PATH`. The JSON report includes the measured commit, binary SHA-256s, every load reading and each run. A missing or extra receipt or a mismatching release report fails the measurement.

Each workspace reports output bytes (candidate receipts and released SARIF) and retained store bytes/files. Each candidate starts in an empty directory. The release starts without a product store. Both inventories are taken after the process exits, before cleanup. Byte counts use logical file sizes. The temporary SDK trace and timing logs belong to the benchmark driver. Every repeat retains its counts in the JSON.


## Workspace memory scale

`scale.py` prepares 1×, 2× and 4× workloads from the same captured check. Each copy has a distinct repository and absolute input roots. Relative paths and cross-file project boundaries are preserved. The replay submits every request to one session. The live benchmark runs the normal check for each root with one shared analysis worker and retains the session across roots, as a long-lived caller would. One-process peak RSS is the gate.

Build the linked worker with `../project/live/link-release.py`. Prepare the fixture in a fresh directory using `scale.py prepare --help` and the pinned live worktree, replay manifest and authority mapping. For each size run `scale.py run --help` once per mode under the same positive byte budget. Use a single CPU and reserve a large-memory slot for 2× and 4× while required. Measure baseline growth without applying the gate. For a candidate use the baseline's `--live-oracle` and original `--receipts`. The scale runner verifies all receipts per copy and the full live SARIF after normalizing only repository paths.

`scale.py gate --output SERIES` passes when each mode answers the expected number of requests and its 4× RSS falls within 1 MiB or 0.5%, whichever is larger, of its 1× RSS. Pass `--live-peak-limit-mib` with the measured peak of the smallest workspace to require every live size to fit that reference with the same tolerance. Each run records process CPU, the Ryo phase clocks, load and peak. Heap profiles are separate runs with `--heap`.
