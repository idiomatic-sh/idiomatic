# Leak review: claude/clippy-b at 720b2fc0b0

Source 720b2fc0b02102fcd9d788810dd4b5f43988c8e3, base 877275be9e. Commits 7b31c4ab07 (core) and 720b2fc0b0 (execution). Closure: ryo core, ryo execution and the crates they import or are imported by. No behaviour change found.

## 1 Proof where the contract lives: PASS
The diff touches no file under `benchmarks/` or `languages/`. Every proof that changed stays in its owning crate.

## 2 Every enforcement claim fires: PASS
- The `#[cfg(test)]` marks at `execution/tests/test.rs:39,250-279` and `tests/activation/test.rs:3` hide nothing. Cargo passes `--cfg test` to the `harness = false` target `activation` (`execution/Cargo.toml:59-62`, seen in the verbose rustc line) and passes `--test` to `boundary_contract`. A type error planted in each of `activation/fixtures/program/mod.rs`, `boundary_fixture/mod.rs`, `union_rows/mod.rs` and `file_run/mod.rs` was reported every time (4 of 4), and each plant was reverted.
- `acceptance.rs:19-48` now returns `Result`. `tests/bdd.rs:160-171` fails on `execution_has_failed` and asserts the exact passed-step count, so an `Err` fails the scenario, as the old `expect` panic did.
- The test wildcards that became named arms list exactly the remaining variants, after the same guarded arms, so no panic arm became reachable or unreachable.
- Advisory: the cited clippy lints were already denied workspace-wide. The plants for them were staged but not run, because `guard_shell.py` refuses clippy outside an integration tree. The integrator's clippy run on the merge is the evidence that every `#[expect]` is fulfilled.

## 3 Boundaries hold across crates: PASS
No new `pub` or `pub(crate)` item, and no visibility, manifest, feature or import change. The touched fixture files gain only docs or typed conversions, behind their existing gates.

## 4 One implementation per concept: PASS
Every change rewrites an existing expression in place. No helper was copied into a test.

## 5 Layer direction: PASS
No new import crosses a layer. `acceptance.rs` names only the inbound `OperatingError` of the use case it already drives.

## 6 Test seams stay out of production: PASS
Every added `#[cfg(test)]` is in an integration test crate, never under `src/`. All targets of both crates compile clean.

## Behaviour hunt
- **Refusals:** the new `usize::try_from(file.0).map_err(|_| OperatingError::NotRun)?` (`evaluation/service.rs:96,212,221`) converts a `u32` `FileId`, which cannot fail on a 32- or 64-bit target. No new `NotRun` is reachable.
- **`elapsed_ns`:** the trace is skipped only when a parse runs longer than `u64::MAX` ns (about 584 years), where the old `as u64` truncated. Trace output is unchanged for every parse that can happen.
- **Production wildcards** (`publication.rs:65`, `evaluation/service.rs:153`, `project/service.rs:193`): each lists the remaining variants after the same guards, and the compiler checks exhaustiveness.
- **`#[expect]` reason** (`rule_compilation/service.rs:128-143`): true. `observation_schedule/service.rs:68-83` marks each rule done once and returns `Cycle` when no rule is ready.
- **Digest `% 256`:** over `0..count`, an unsigned `index % 256` is the same byte as the old `index as u8`, so the digests are identical.

VERDICT: PASS
