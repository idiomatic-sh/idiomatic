# Leak review: claude/review-criteria

Source `b9fc93cf21c3bf51bdee2a1942f4c84c8534c3a3`, base `e75e47b059`. Docs-only diff: `.agents/agents/review-criteria.md` (new) plus one "Grade against" line in each of `rust-principal-review.md`, `contract-review.md`, `leak-review.md` and `tm-specialist.md`, and the matching edits to the `checks` sentence. All evidence below was read with `git show b9fc93cf21:<path>`.

Tree note: the worktree `/workspaces/ryo-criteria` was at `b9fc93cf21` at the start of the review and at `cd8d324dfc` at the end. Two commits that touch the hooks, `builder.md` and `tm-specialist.md` landed on top during the review. This review reads only the pinned sha and never wrote to the tree.

## Before and after, check by check
No existing check was dropped, weakened or reworded. The only rewordings append text: the `checks` sentence of each reviewer now adds the own-check ids (rust :47, leak :45, contract :27), and every original FAIL/PASS row is byte-identical to the base. Coverage: every item 1-21 has at least one owner, and every owned item is either mapped or graded as its own check (Rust 19 and 21; leak 16, 17 and 18; contract 10, 15, 19 and 21; TM 9 through step 3). No item number is left without an owner.

## 1 Proof where the contract lives: PASS
No contract is asserted or proved anywhere in the diff.

## 2 Every enforcement claim fires: PASS
The diff cites no rule, probe or lint and touches no test or runner. `shape()` (`ci/contract_review/service.py:28-53`) puts no limit on check ids or their count, so the added ids validate. This file uses 16-18 and passed `shape(..., "leak-review")`. No other consumer in `ci/` or `.claude/hooks/` counts checks or fixes their ids.

## 3 Boundaries hold across crates: PASS
No code. `.agents/agents/{*agents}` is an unchecked placement (`tastemaker.toml:19`).

## 4 One implementation and one representation per concept: FAIL
`review-criteria.md:2` says it is "the one definition of what every reviewer grades". Each reviewer's FAIL/PASS table is still the text it actually grades against, and several mappings say a check covers an item that its table does not state. That leaves two definitions of one check that disagree, so the parts that differ go ungraded. Every instance in the class:

- **Rust 1 -> 12, 13, 14 and 7 -> 12** (`rust-principal-review.md:20`). Item 1 (`:12`) covers borrow-checker fights and the Public SDK contracts list (`:49-57`), which applies to the SDK surface only. Item 7 (`:18`) covers unsafe, unwrap, panics and bare `#[allow]`. Neither covers these parts of item 12: `as_`/`to_`/`into_` naming, `From`/`TryFrom`, derived traits, `#[must_use]`, iterators, or typed errors and builders outside the SDK. Neither covers these parts of item 13: semver checks run, features additive. Neither covers these parts of item 14: `# Errors`/`# Panics`, crate- and module-level docs, tested doc examples. Worse, `:60-75` makes exactly these topics advisory, and `:69` says they "never change the seven-item verdict". `:75` makes semver "advisory until distribution". So the mapping and the advisory section contradict each other.
- **Rust 3 -> 11**. Item 11 requires "perf claims from matched, quiet measurements". Hot path (`:14`) does not state it. Contract Proof (`contract-review.md:16`) does state it, but contract review does not own item 11.
- **Rust 2/7 -> 9**. Item 9 requires "every suppression ... actually suppresses something". No Rust item states that, and TM step 3 decides a mechanism, not whether a suppression is live.
- **Rust 4 -> 6 and leak 4 -> 6**. Item 6 requires "one authority per policy (e.g. Tastemaker owns layout and re-exports)". Neither table states it.
- **Contract Enforced boundaries -> 1** (`contract-review.md:14`, `:18`). Item 1's check (concept hex plus wiring, ratcheted census rows don't rise) is not in the criterion, which covers privacy and Tastemaker preventing bypasses.
- **Contract Clear ownership and flow -> 2** (`:15`). Item 2's check (no parallel route, old code deleted in the same change) is not in the criterion.
- **Contract Clear ownership and flow -> 5, Proof -> 7**: only partly stated. The criterion does not state "domain without I/O, imports inward, no common/utils, no flat layouts". Proof does not state "Ruff parity, Django byte identity, refusal text and error order".
- **Leak mappings** (`leak-review.md:19`): 3 -> 3, 13, 20 and 6 -> 13, 20 hold for visibility and seams. Item 20's placement clause (Tastemaker placements, hex layer, concept dir) is in neither leak check 3 nor leak check 6. Contract review co-owns item 20 but grades it only through Enforced boundaries, which does not state placement either. As a result, nobody grades item 20's placement clause. 5 -> 3 and 1/2 -> 4 are partial too: leak check 2 covers vacuous tests and enforcement firing, but not item 4's planted-bug table for seam behaviour. Contract Proof co-owns item 4 and covers it only as "evidence covers every route".

Required change, which closes the whole class at once: add one sentence to `review-criteria.md:2`, and repeat it in each "Grade against" line, saying that a check mapped to an item FAILs on any clause of that item's **Check** line in addition to its own FAIL text. Then delete or reconcile `rust-principal-review.md:69` and `:75`, so that items 12-14 can FAIL item 1 instead of being advisory only. The other way to fix it: grade every partly covered item as its own check (Rust 11, 12, 13, 14; contract 1, 2; leak 20). Either way, the reviewer's tables and the criteria file must stop disagreeing.

## 5 Layer direction: PASS
No code.

## 6 Test seams stay out of production: PASS
No code.

## 16 Dependency hygiene: PASS / 17 Portable and reproducible: PASS / 18 Use what everyone uses: PASS
No dependency, pin, code or tool in the diff.

## Advisories (no verdict effect)
- **Agent loading.** `.claude/agents` is a symlink to `../.agents/agents`. `review-criteria.md` is the only file there without YAML frontmatter; every other file starts with `---` and has `name`/`description`. Claude Code skips an agents-directory `.md` that lacks a `name`/`description` frontmatter. The skipped file can show up as a parse error in `/agents`, but the other definitions still load. What was tried: `claude agents` only lists background sessions and needs a TTY, so it did not settle this. The probe that would settle it is opening `/agents` in an interactive session at this sha. The file would be cleaner outside the agent directory, for example under `crates/ryo/docs/contracts/review/`, linked from each reviewer.
- **Shipped text.** No machine paths, tracker ids or profanity in the new text. Item 10 uses "the fused rewrite" and "the goal's out-of-scope list". Both are internal references that a stranger cannot resolve, which is item 15's own test.
- **Verdict wording.** `contract-review.md:22` still says "All three must pass" and `leak-review.md:8` still says "each of the six checks". The added own-checks are covered mechanically, because `shape()` refuses a PASS that has a FAIL check, but the prose should say "every check".
- **Duplicated ownership list.** Item ownership is written twice: once in the owner table at `review-criteria.md:4-9`, and again in each reviewer's "Grade against" line. The two copies agree today, but nothing checks that they stay in agreement.

No enforcement candidates: every finding is semantic.

VERDICT: FAIL: 4
