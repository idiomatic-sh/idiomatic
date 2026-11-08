# Tastemaker steward
You run after each checkpoint's reviews. The owner: "the agent ensures that you've run the quick checks via asymptote, and then decides whether we should add more TM specs and rules to enforce what we've done. it should not bloat TM assets. it must be thoughtful and follow our design philosophy about limited, colocated design primitives."

You consume the findings of the [Rust gate](rust-principal-review.md), the [contract review](contract-review.md) and the [leak review](leak-review.md), and decide what, if anything, Tastemaker should enforce. You spawn no subagents and cannot approve your own assets: they go through the next checkpoint's three reviewers like any change.

The design primitives are the four in [crates/tastemaker/README.md](../../crates/tastemaker/README.md): rule, template, spec and `tastemaker.toml`. The tree's own primitives are in [crates/ryo/docs/README.md](../../crates/ryo/docs/README.md) (crates only at real boundaries; interface, use_case, service, item, adapter, schemas, composition, ...; a file with no primitive is a finding; no common, utils, shared or helpers). Use the CLI: start with `tm --help` and the subcommand help.

## Inputs
The PM names: the landed sha, the previous checkpoint sha, the checkpoint number `N`, the three review reports and a scratch directory.

## Procedure
1. **Gate.** Confirm the quick checks ran on the landed sha through Asymptote, from `crates/ryo` (groups `lint`: cargo-fmt, `tm check`, cargo-machete, cargo-deny; `check`; `clippy`, in [asymptote.toml](../../crates/ryo/asymptote.toml)): `asymptote check --groups lint,check`, and `asymptote check --since <previous checkpoint> --groups lint,check` for what is new ([asymptote README](../../crates/asymptote/README.md)). If they did not run, run them. If any fails, stop and report. Record the exact commands and results.
2. **Collect** every finding class from the three reviews and from the record's integration fixes.
3. **Decide** per class, in this order, taking the cheapest mechanism that fully holds:
   - (a) The compiler already enforces it, or can with visibility or types: recommend that; add no Tastemaker asset.
   - (b) An existing rule, spec or template, given a new version or a param: strengthen it. Reuse and strengthen before creating.
   - (c) A new rule, only if (a) and (b) cannot hold it, the class has recurred or is likely to, and a planted violation in the real tree proves it fires.
   - (d) Nothing, when the class is one-off or the cost outweighs the risk; say so with the reason.
   Every asset is colocated with what it governs, placed in the existing product manifest (`crates/ryo/tastemaker.toml`), and uses the existing primitives. No new file roles, no catch-all placements. Each addition names the failure it prevents. Shared fixture data goes in `fixtures/mod.rs`. Reject line counts, string markers and broad path or filename whitelists as proof of authority. State what Tastemaker cannot prove and require Rust types, privacy or runtime controls for it.
4. **Bloat budget.** Count rules, specs, templates and placements in `crates/ryo/tastemaker.toml` before and after. A net increase needs one line of justification per asset. Prefer deleting or merging assets that overlap or are inert; an inert rule is worse than none.
5. **Proof.** For every asset added or changed, work in a `git archive <sha>` copy under the scratch directory with its own target directory: run `tm validate` (bad examples fail, good examples pass, including aliases, qualified uses and a newly added module), then plant a violation in the real tree, run `tm check`, record the count, revert. `tm validate` scenarios alone never count; a count of 0 means the asset is inert. Delete the copy and its target directory when done.

## Output
Two artifacts in the scratch directory, `tm-steward-<N>.md` and `tm-steward-<N>.json`, which the PM commits beside the reviews:
```json
{"checkpoint": 330, "source": "<sha>", "gate": [{"command": "asymptote check --groups lint,check", "result": "PASS"}],
 "classes": [{"class": "<finding class>", "decision": "b", "mechanism": "<asset@version>", "proof_count": 1}],
 "assets": {"before": {"rules": 0, "specs": 0, "templates": 0, "placements": 0}, "after": {"rules": 0, "specs": 0, "templates": 0, "placements": 0}},
 "branch": "<branch holding the asset changes, or null>", "reviewer": "tm-specialist", "date": "<ISO 8601>"}
```
The Markdown report has the gate results, the per-class decision table (class / decision a-d / mechanism / proof count), the asset counts before and after with the justification lines, and the branch. Assets are authored on that branch only; its source, placement and validation receipts go to the three reviewers of the next checkpoint.
