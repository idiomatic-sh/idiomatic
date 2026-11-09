# Leak review: claude/p7-skip-rules (re-review)

Source `8eeb39f8ed3f016d44d457e06b5ae66398f7c0b9` (code at `db950ed280f21d541aa58fc2f1f4fb02e4bc08fb`), base `1086daee4b3a4435e60438d2df6ec7a7128dd883`. The first review, of `33b5c2bb2c`, was FAIL: 2.

## Check 1 Proof where the contract lives: PASS
Unchanged. The proofs are engine-owned and language-free.

## Check 2 Every enforcement claim fires: PASS

### (a) Literal patterns
`invalid_patterns` now matches every `Operation` variant, with no wildcard arm. Replace and SourceMatches make input 0 fallible when their literal pattern does not compile (`presence/narrowing.rs:93-95`).

R6 is now tied to the operations that raise each error. Every row-dependent verdict lists its raisers; `REGEX` includes `Pattern(Replace)` and `Pattern(SourceMatches)` (`presence/service_test.rs:437-445`). `every_fault_has_a_verdict` asserts that each raiser declares its input, through `prepare_for` or `invalid_patterns`. The oversized-pattern case is included.

**My Replace plant, re-run on `db950ed280`:** pattern `"("` in the Symbol Replace, on the file where its join conjunct is absent.

| Head | Result |
| --- | --- |
| `33b5c2bb2c` | 0 rows, no error |
| `db950ed280` | `InvalidRegex` |

The test's own next assertion ("the Symbol Replace never ran") then fails, which is the expected sign that the rule now runs. Two new tests cover the case:
- `an_invalid_replace_pattern_is_reported_where_its_input_has_rows` (`presence_service_test.rs:567`);
- `an_invalid_select_pattern_is_reported_where_its_input_has_rows` (`:589`).

### (b) The presence read is admission's first read
- **Successful read:** presence pins it through `RelationOwner::source` (`file_execution/presence/service.rs:25-37`). Admission's later read is checked against the pin, so a changed population is refused. The `Changed` fixture is back to `> 1`, with `calls [2, 0]`.
- **Refused read:** presence records the refusal. Admission returns it without calling the provider again (`source_admission/acquisition/service.rs:48-50`, the only admission read site).
- **Assertions:** the one-read assertions are restored in `union_rows` (left refusal), `provider` `Missing::First`, `axis` and `readiness`.

### (c) Kernel pins
The SpanBetween `continue` is removed. `compare_emptied` (`conformance_driver/test.rs:34`) empties each input as `Difference(A, A)`. Its presence is A's presence, so the region runs and the kernel itself answers, for every kind.

### The two adapted order checks: accepted
The checks are `provider` `Missing::Second` (`provider/test.rs:186-206`) and the right-refusal branch of `union_rows` (`union_rows/mod.rs:288-300`). They no longer require the refused right side to be read last.

They still require:
- the left side read first;
- exactly one right-side read;
- no other relation read.

The dropped property is about admission's lazy demand. Presence reads before admission, in formula order, while the single-read and error assertions still bound the provider calls and the outcome.

## Check 3 Boundaries hold across crates: PASS
The `AbsentRoots::decided` advisory is closed with a reason: `execution_runtime` constructs it, so `pub(crate)` cannot serve.

## Check 4 One implementation and one representation per concept: PASS
There is one source-read path: the pin and the refusal record are shared with admission.

## Check 5 Layer direction: PASS
`RegionCount` moved from `readiness` into `executable_program` types, which both the fold and the runtime use. Domain does not reach application.

## Check 6 Test seams stay out of production: PASS
`SourceRefusal` is production state that admission reads, not a test seam. The test probes stay `#[cfg(test)]`.

## Advisories
- `relation:Annotates` stays in `unfed.toml` until a replay file covers it. The reason is accepted, and the item stays open.
- Still open from the first review:
  - the probe checks that a read succeeds, not that its rows can be read;
  - an interior-phase refusal keeps the interior marks made before it.

## Enforcement candidates (carried forward)
| Candidate | Instance |
| --- | --- |
| `literal-pattern-missing-from-failure-dc2bb9e8` | fixed |
| `presence-read-unbound-from-admission-e5ef354a` | fixed |
| `test-loop-skips-a-kind-cd234251` | fixed |
| `shrink-only-list-grows-69954a9d` | open |

## Process note
The plant ran in an archive copy of `db950ed280`. The hook refused one other `cargo test` call, before the plant run. The copy and its target directory are deleted.

VERDICT: PASS
