# Engine consolidation review

Reviewer brief for the integration branch `claude/ryo-consolidation`, which
carries the engine consolidation toward the phases in
[the matrix](../README.md). One phase is active at a time; integration is never
red except for the known-red list below.

## Branch

Checkpoints 310-331 are evidence commits on the branch; each names the batches
it merged and the validation it ran.

| Checkpoint | Sha | Landed |
|---|---|---|
| 310 | `18fe9b6916` | sparse-activation, route-coverage and tm-fixture batches |
| 311 | `d92fce4796` | Phase 2 scheduling (structure, submission, lifecycle) |
| 312 | `51aeb2cf4b` | Phase 3 shared-walk batches |
| 313 | `4966f7e3f9` | Phase 2 review repairs, runtime structure |
| 314 | `cd2b23f100` | Phase 4 derived ownership, Phase 3 repairs |
| 315 | `7e93be040a` | Phase 2/3 repairs, rule re-version, Phase 5 fusion and route |
| 316 | `1e3eba5c90` | Phase 6 reuse and invalidation, Phase 4 census and bypasses |
| 317 | `d3f16fcd11` | Phase 7 TM batches (whole-tree TM 8,060 to 4,251) |
| 318 | `80f473c67d` | Phase 1 conformance and syntax poisoning, Phase 2 to 6 review repairs III, blow-up replay repairs, Phase 7 operators/core TM (validated source `c043a6dc68`; whole-tree TM 3,676) |
| 319 | `f2d8aaf1db` | the frozen 318 branches (whole-tree TM 3,676 to 2,762) |
| 320 | `61cb444739` | wave 2 (whole-tree TM 2,762 to 1,223) |
| 321 | `0015475b6e` | wave 3 and the benchmark branch (whole-tree TM 1,223 to 34) |
| 322 | `d13686afb5` | wave 4 (whole-tree TM 34 to 24) |
| 323 | `7d93840b19` | perf iteration 1, enforcement rules, ratchet 321, live hooks, review repairs VII |
| 324 | `14a8352416` | landing 1 (execution_domain and execution_store crates), review repairs VIII, harness 324 |
| 325 | `dcd94840fc` | review repairs IX, Django unknowns second pass, ratchet 323 |
| 326 | `f3a9a7e95c` | review repairs X, perf iteration 2 |
| 327 | `e49dc1dfc5` | review repairs XI, perf iteration 3 (validated source `142fe40a98`) |
| 328 | `90a0a472b2` | landing 2 (the runtime crate), Phase 1 tail, review repairs XII |
| 329 | `1fce372156` | review repairs XIII, runtime follow-up, engine fixes after landing 2, heap profile feature (validated source `74a799c116`) |
| 330 | `1086daee4b` | perf iterations 4 and 5 (Django peak 6.78 to 3.33 GB; validated source `4df6abaca2`) |
| 331 | `b1e81f7242` | perf gate, typed unknowns, repairs XIV, sharing contract, leak repairs A, store key, lineage, p6 diagnosis (validated source `340808449f`) |

## How work lands

Builders write uncompiled batches on side branches. The integrator merges them,
compiles once, runs the committed validation set, and lands only green. The
checkpoint commit message carries the numbers. Review the exact merged source,
never a branch tip.

## Evidence

[`evidence/checkpoint/validation.json`](../evidence/checkpoint/validation.json)
and `identity.json` (source revision plus the command list) are committed; the
set is described in the [checkpoint README](../evidence/checkpoint/README.md).
Raw logs live under `/tmp` per run and are never committed.

## How reviews run

The [contract-review agent](../../../../../.agents/agents/contract-review.md)
runs each review. Each phase writes its verdict as the
[review evidence schema](#review-evidence-schema) file under
`audits/2026-11-06-phase<N>/`. A BLOCKING verdict gates the matrix tick for its
rows, not the merge. Source changes invalidate a verdict; pin a fresh source
sha each round.

## Requesting a review

A request names both reviews: the contract review above and the
[rust-principal-review agent](../../../../../.agents/agents/rust-principal-review.md),
which scores seven Rust items PASS or FAIL on the same merged source and
writes `evidence/checkpoint/rust-review.json`. A FAIL from either stops the
landing. A request carries three receipts, and a request without all three is refused by
the reviewer, because what a linter or check catches is resolved before a
person reads the source:

1. `tm --root crates/ryo check <the review's scope directories> --format json`:
   tm over the review scope is 0 after excluding findings each named in the
   request with its owning OPEN design row (row id, rule, count); an unnamed
   finding fails the receipt.
2. `python3 crates/ryo/docs/contracts/evidence/checkpoint/run.py --check` exits 0.
3. `validation.json` records the `no_print` clippy target green and file-level
   ownership cycles 0.

The request points at the checkpoint's integration record, and the record's
pass/error table is part of it: one row per `cargo check` pass with the errors
that pass reported and the errors it fixed.

## Review priorities

The five adversarial bullets in the agent file apply unchanged: callers and
bypasses, authority and identity, derived-kind ownership, shared-walk coverage
of the configured 413 rules, and real positive and negative tests with TM
enforcement. See "Adversarial pass" and "Architectural enforcement" there.

## Builder test rule
Every fix names the test that fails before it and passes after, in its commit message or branch notes. A fix is a change to production behaviour: docs-only and evidence-only changes are exempt, and perf branches follow the [perf diagnosis](../../../../../.agents/agents/perf-diagnosis.md) `after.json` rule instead. The integrator records that test red, then green, in the [red-then-green record](../evidence/checkpoint/README.md#red-then-green-record); the Rust gate item 6 fails a fix whose commit names no such test. A fix to a harness repairs the harness; it is not covered by a parallel test beside it. The worker agent [contract-migration](../../../../../.agents/agents/contract-migration.md) carries this rule.

## Review evidence schema
Every review evidence file (rust-review, contract-review, leak-review, tm-steward and the per-branch leak result) has this shape, and `.claude/hooks/guard_shell.py` refuses a file that does not:

```json
{"source": "<40-hex>", "verdict": "PASS"|"FAIL"|"NOTE"|"BLOCKING", "reviewer": "<subagent name>",
 "checks": [{"id": "...", "verdict": "...", "evidence": "..."}], "findings": [...]}
```

Evidence JSON carries no tracker ids (the no-tracker-text-shipped check flags them): an advisory in `findings` reads `open item: <short title>`, and its bead id goes only in the Markdown report, where the PM files the bead.

`source` is the full sha reviewed, never a ref name. `reviewer` is the agent's frontmatter name (`rust-review`, `contract-review`, `leak-review`, `tm-steward`); `evidence/checkpoint/asymptote.json`, the integrator's quick-check record, is the one file whose `reviewer` is `integrator`. `checks` is a list (the items, criteria or gate results of that agent); further fields (`checkpoint`, `date`, `plants`, `gate`, `classes`) may sit beside them.

A `leak-review` file may also carry `enforcement_candidates`, optional and a list (empty or absent when no failing or advisory finding has a mechanical signature). Each entry is `{"id": "<id>", "class": "<finding class>", "signature": "<the exact code shape a rule would match>", "scope": ["<path>", ...], "suggested_mechanism": "compiler" | "strengthen <existing rule id>" | "new rule", "plants": [{"file": "<path>", "line": 1, "text": "<planted line>"}]}`. `id` is the class as a lowercase hyphen slug, a hyphen, then the first 8 hex digits of the sha256 of `signature` (`candidate_id` in the validator). Optional `instance_status` is `fixed` or `open`. Candidates are append-only: a re-review carries every earlier id forward (the push hook refuses a modified branch file that drops one). `scope` is a non-empty list of paths; `plants` is a list; no entry carries a tracker id.

The `tm-steward` file then carries `candidate_decisions`: `[{"id": "<candidate id>", "decision": "compiler" | "strengthen" | "new" | "none", "mechanism": "<asset@version or compiler feature>", "plant_counts": [1], "reason": "<why>"}]`. A decision other than `none` needs a mechanism and a non-empty `plant_counts` of integers that are all at least 1; `none` needs a non-empty `reason`. The consolidation push hook (`previous_reviews`) refuses a push unless `tm-steward.json` decides every candidate id found in exactly these files: at the push of N+1, N's `leak-review.json` plus the `evidence/branches/*.json` files added or modified (git status A or M) during N+1. Decisions do not carry forward. `shape` validates both fields when present.

Reviewers write evidence directly into the repo, never to a scratchpad. The PM names `INTEGRATION_TREE`, the open checkpoint integration worktree, in each prompt: a per-branch leak result goes to `<INTEGRATION_TREE>/crates/ryo/docs/contracts/evidence/branches/<branch-dashed>.json`; the checkpoint results for landed N (`contract-review`, `leak-review`, `tm-steward`) go into checkpoint N+1's tree under `evidence/checkpoint/`; the Rust gate result for N goes into N's own tree as `evidence/checkpoint/rust-review.json`. The report is `<name>.md` beside each JSON. A reviewer writes files only and never commits, and stays read-only on the code it reviews; the integrator commits the files with its next commit, and the hooks keep validating the committed copies.

The write guard refuses writing a `rust-review*.json`, `contract-review*.json`, `leak-review*.json`, `tm-steward*.json`, `asymptote.json` or any `evidence/branches/` file anywhere but a `claude/checkpoint-N` worktree's `crates/ryo/docs/contracts/evidence/`. `.claude/hooks/guard_shell.py` covers these Bash write forms: `>`, `>>`, `tee`, `cp`, `mv`, `install`, `rsync`, `ln`, `sed -i`, `perl -i`, `sponge`, `dd of=`, and `python3 -c` code that opens a file for `w` or `a` or calls `json.dump`, `write_text` or `write_bytes`. `.claude/hooks/guard_write.py` covers the Write, Edit, MultiEdit and NotebookEdit tools through a `PreToolUse` matcher `Write|Edit|MultiEdit|NotebookEdit`, committed in `.claude/settings.json` as `python3 -I "$CLAUDE_PROJECT_DIR"/.claude/hooks/guard_write.py` (the machine-level settings run the same file from the hooks worktree). Allowed: a variable-rooted path into an evidence directory, such as `"$INTEGRATION_TREE/crates/ryo/docs/contracts/evidence/..."` (the variable is expanded from the hook's environment first; an unexpandable root is allowed only when the rest is an evidence path); a file named `*-committed-copy.json` outside `evidence/branches/`; and `git show <rev>:<committed path> > file`, a read copy of committed content.

One definition validates this shape: `shape` in `ci/contract_review/service.py`. Its three callers are `review_shape` in `.claude/hooks/guard_shell.py` (which loads it by path and returns its message; it checks every push and merge that reads evidence, including `evidence/checkpoint/asymptote.json` when present), `shape_problem` in `.claude/hooks/acceptance_on_stop.py` (which holds a review agent's stop until its verdict file passes), and the `contract-review` CI job. `VARIANTS` in `ci/contract_review/service_test.py` is the behaviour table; the `ci-tests` tool reruns it when either file changes.

## Known red

Each is named with its reason and owner in `validation.json` (`results`, validation332, validated source `7fd2dbf1f5`) and in the
checkpoint 332 integration record in [the records](../records.md).

- `core_allocation` (core `allocation` test, exit 101): a Phase 7 measurement.
- Benchmark library 227 / 3: `verify_cases` `process_capture::test::succeeds`, one red row, and the parity tool's `compare_corpus` `use_case_test::{refuses,succeeds}` (each answers an operational failure over the fakes).
- Cli `replay` target: 96 passed / 3 failed. The three causes span-semantics' stricter harness exposed are repaired: the replay
  links the TOML language beside the Rust language, so the three manifest recordings run and give their recorded rows; the
  manifests tests write a missing value as absent; a located compile refusal is matched by its innermost refusal. Left, owned:
  import-fan-in's false `derive_message_formats.rs` edge, an `error-variant-never-constructed` row for a variant `?` constructs
  through `#[from]`, and the warm peak.
- Cli `cli_cost`: 4 / 3, the three unseeded ruff ratchet rows (cold 9,331 ms, peak 579 MB, warm 629 ms); the Django rows answer
  not available without a preflight worker.
- `tm_import_cli`: Tastemaker's ryo adapter now fills the local SDK's request fields, so it does not compile against the published
  SDK 0.1.1 the replay resolves.

## Verification limits

Not covered by the latest validation (`validation.json`, validated source `7fd2dbf1f5`, checkpoint 332, validation332; "not full
product acceptance"):

- Whole-tree TM is measured with the rebuilt tm 0.1.1, which reads use-site paths as imports: 1,467 findings, 13 suppressed:
  1,464 `#imports` false positives of that reading (the tm 0.1.2 branch fixes it) and 3 in untracked reviewer drafts; the
  findings 332 had introduced are fixed (331 under 0.1.0: 18, the domain-concept ring, now 0). The scoped `tm_source` run is 0.
- The receipts for the root and Tastemaker workspaces use tm 0.1.0 (their pin); crates/ryo uses 0.1.1, unpublished.
- No new performance claim: the Django receipt is a store/peak record for the span-semantics attribution, not a speed claim.
- Language conformance targets print no pass count; exit 0 is the receipt.

Links: [matrix](../README.md), [execution plan](../execution-plan.md),
[reviewer instructions](../../../../../.agents/agents/contract-review.md),
[Rust reviewer instructions](../../../../../.agents/agents/rust-principal-review.md).
