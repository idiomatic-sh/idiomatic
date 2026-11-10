# Leak review: claude/w14-noqa-fragments

Source `555076902b1992304c17b00098b2e90ed0c9bb9f`, base `79ef18b419d4cafe87305f374b714cb109b2a8ad`. The diff changes Ruff benchmark assets only: no engine, provider or CLI code.

## Check 1 Proof where the contract lives: PASS
No engine contract is asserted from the product directory.

## Check 2 Every enforcement claim fires: PASS

**One grammar.** RUF100 and RUF101 import `noqa-heads`, `noqa-codes` and `noqa-blankets` (`rules/RUF100/query.ryoql:1-8`). RUF100 computes its own `span-between` (`:291`).

**Codes match Ruff.** A script over all 418 rules found:
- 414 rules in the v2 policy, each with exactly one `codes=` equal to `render_codes(...)` (0 mismatches);
- the 4 rules outside it are exactly E902, RUF100, RUF101 and RUF200, and none has a `codes=` call.

Probes with Ruff 0.16.5 agree with the exact-spelling rule:

| Directive on a UP031 line | Ruff suppresses UP031? |
| --- | --- |
| `# noqa: UP031` | yes |
| `# noqa: U031` | no |
| `# noqa: U0` | no |
| `# noqa: UP` | no |
| `# noqa: U` | no |

Ruff never suppresses RUF101 by noqa, so its exemption is correct.

**Guard v2.**
- The self-test passes, and its refusals can fail: the shared helper raises if a bad form is accepted.
- The live check over the real tree accepts all 414 rules.
- Plants on B035, in a copy:

| Plant | Guard |
| --- | --- |
| codes widened to `^(?:B035\|RUF011\|PIE999)$` | refused |
| codes narrowed to `^(?:B035)$` | refused |
| emit no longer depends on the suppression family | refused |
| antijoin replaced by `union(rows, matches)` | **accepted** (advisory) |

**Narrowed grammar ban.** `(?i:noqa)` is refused only together with `capture=` (`profile_proofs.py:258-259`). E501's pragma test stays legal, as the self-test pins.

## Rebinds: every one tracks a byte change
- **profile-sha256:** each of the 267 old→new pairs in the changed `cases.toml` (40 distinct) equals the base→head sha256 of one of the 335 changed profile `benchmark.toml` files. None is unexplained.
- **Profile edits:** only `query-modules` declarations changed.
- **Other `cases.toml` changes:** only the 14 appended new cases; no existing expectation changed.
- **UP031:** the 8 `invocation.json` files change only `profile_sha256` plus an appended `context_derivation` note.

The builder reports 285 rebinds. I count 267 sha lines, all checked, so the gap hides no change.

## Spot checks
- **Reference side:** I ran the 14 new cases through Ruff 0.16.5; all 14 equal their expected rows.
- **ryo side, not re-run (residual):** the claims of 412/413 parity with `per-rule.json` byte-identical, Django `f50bb571`, and RUF100 29/30 need a sealed worker. None remains on disk, and the hook refuses building one. I made one attempt, a disk search for a worker, and found none.

## Skip gate
The 87.76% value is a recorded give-back (design §7, `after.json`). No code gate in this diff enforces it. Restoring it is tracked separately.

## Check 3 Boundaries hold across crates: PASS
No crate visibility, import or allowlist boundary changes.

## Check 4 One implementation and one representation per concept: PASS
The grammar exists once, and the spellings come from one generator. The pattern guard and the 12 inline copies are deleted.

## Check 5 Layer direction: PASS
The fragments import downward only.

## Check 6 Test seams stay out of production: PASS
None.

## Advisories
- The guard checks that a rule's emit depends on its suppression, not that it subtracts it. Candidate `suppression-guard-accepts-non-subtraction-361ac554`.
- Nothing checks that every rule importing a suppression module is listed in the policy. Today's exempt set is correct. Candidate `suppression-policy-completeness-unchecked-a5132b93`.

VERDICT: PASS
