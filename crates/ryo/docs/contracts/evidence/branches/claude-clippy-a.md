# Leak review: claude/clippy-a at f3e9e08f14 (re-check of c8347684cd)

Scope: the fix commit f3e9e08f14 on top of the FAIL-reviewed c8347684cd (one commit: crates/ryo/Cargo.toml, benchmark/src/outbound/suite/fixtures/fake_test.rs, query_language/src/domain/planning/service.rs; 7+/1-). Closure as in the previous review. Trees unchanged before and after (clippy-a tree HEAD f3e9e08f14, clean; checkpoint-334 tree HEAD f5d1718a06).

## 1 Proof where the contract lives: PASS
Unchanged since c8347684cd; the fix adds no proof in benchmarks/ or languages/.

## 2 Every enforcement claim fires: PASS
- benchmark/src/outbound/suite/fixtures/fake_test.rs:8 now unwraps `check(...)`, so a setup Err fails the SuiteFake contract test.
- crates/ryo/Cargo.toml:96 `unused_must_use = "deny"` overrides the `unused` warn group (priority -1). All 26 workspace members inherit workspace lints.
- Plants in a git archive copy: dropping the unwrap again gave 1 error (`fake_test.rs:8:5: unused std::result::Result that must be used`). A dropped `"1".parse::<u8>();` in production code at planning/service.rs:270 gave 1 error. Both plants were reverted.
- After the reverts, a compiler check of every target of all 26 members (one `-p` per member) gave 0 errors. This confirms the builder's claim.

## 3 Boundaries hold across crates: PASS
Unchanged; no visibility or feature change in the fix.

## 4 One implementation per concept: PASS
Unchanged. The earlier advisory is resolved: planning/service.rs:270-273 now carries a reasoned `#[expect(clippy::indexing_slicing)]`.

## 5 Layer direction: PASS
Unchanged; no import added.

## 6 Test seams stay out of production: PASS
Unchanged; the unwrap is in a #[test] file.

## Enforcement candidates
- dropped-must-use-result-f839521b: instance fixed, and the class is now enforced by the compiler (unused_must_use deny).
- sentinel-index-hides-panic-62cdc96e: instance fixed (reasoned expect). The TM steward decides the class.

## Advisories
- f3e9e08f14 merges cleanly into 334. 334 has changed 58 .rs files since merge base 124ccc45cf, and the new deny applies to them. The integrator's merged build proves them; a violation there is a compile error, not a silent pass.
- mod_module_files is no longer denied in 334 (ecf7dc5fc9), so the 48 held findings do not arise after merge.

VERDICT: PASS
