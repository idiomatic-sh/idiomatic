# Review criteria
The one definition of what every reviewer grades. Each reviewer definition names the items it owns and maps its own items or criteria to these numbers; an owned item that none of the reviewer's own items maps to is graded as its own check, with the item number as its id. Items 1 to 11 and 18 to 21 are the owner's, verbatim; 12 to 17 extend them to idiomatic, conventional Rust and to developing for open source.

| Item | Owner |
| --- | --- |
| 1, 2, 4, 5, 7, 10, 15, 19, 20, 21 | [contract review](contract-review.md) |
| 3, 6, 8, 9, 11, 12, 13, 14, 18, 19, 21 | [Rust gate](rust-principal-review.md) |
| 3, 4, 6, 13, 16, 17, 18, 20 | [leak review](leak-review.md) |
| 9, and a Tastemaker rule for any recurring class among all 21 | [TM steward](tm-specialist.md) |

1. **Isolation** — a perf change or a concept added or removed is a focused change behind a tested contract; a rewrite is fast because the seam is narrow and tested, not because people are careful.
   Check: inside its concept's hex plus wiring, other touched files justified; ratcheted census rows (edit footprint, cross-crate tests, Operation matches outside the replaced set, fact touch points) don't rise.
2. **One path** — no second implementation, fast path, simple variant or behaviour-forking flag; replace by swapping behind the port and deleting the old.
   Check: no duplicated logic or parallel route; old code deleted in the same change.
3. **Ports and seams** — narrow sealed interfaces, private internals.
   Check: compiler first (visibility, sealed traits, newtypes), then Tastemaker; nothing reaches past a seam, tests included; the public SDK exposes only what a user knows about, under every feature combination.
4. **A tested contract** — tests at the seam, naming no internals.
   Check: every seam-observable behaviour has a planted bug a seam test catches; anything unobservable is argued and recorded; no vacuous tests (guards prove the run did its job, comparisons pin which path ran, no ignored results).
5. **Hex layout and colocation.**
   Check: domain without I/O, thin use cases, imports pointing inward; a concept and its implementations together; no common/utils/helpers, no flat layouts.
6. **One definition per concept.**
   Check: no copied data or repeated computation across layers; one representation per concept; one authority per policy (e.g. Tastemaker owns layout and re-exports).
7. **Byte identity** — anything not meant to change output doesn't.
   Check: parity against Ruff, Django byte identity, replays; refusal text and error order unchanged or the change stated and justified.
8. **Every fix is proven.**
   Check: each behaviour change names a test red before and green after, or a lint or rule disappearing; moves behind a seam proven by the planted-bug table; comment-, lint- and test-only changes are exempt.
9. **Cheapest enforcement first.**
   Check: Rust types and visibility, then standard tools, then existing Tastemaker rules, then a new rule only for a recurring class; no loose ignores; every suppression has a real reason and actually suppresses something.
10. **Don't build what gets thrown away.**
    Check: no work in code the fused rewrite deletes, nothing from the goal's out-of-scope list, no additions beyond the brief.
11. **Fast, correct, easy, in that order of user value.**
    Check: no per-row cost on hot paths; perf claims from matched, quiet measurements.
12. **Idiomatic, conventional Rust** (Rust API Guidelines and std conventions).
    Check: naming (`as_`/`to_`/`into_`, getters without `get_`, `iter`/`iter_mut`/`into_iter`); conversions via `From`/`TryFrom`/`AsRef`, not ad-hoc helpers; common traits derived where sensible (`Debug`, `Clone`, `PartialEq`/`Eq`, `Hash`, `Default`) and `#[must_use]` where values mustn't be dropped; borrow over clone, `&str`/`&[T]` in signatures; iterators over index loops; libraries never panic on caller input; typed errors implementing `std::error::Error` and `Display`, carrying their source; builders or plain constructors, not long positional argument lists.
13. **API stability (semver)** — anything public is a promise.
    Check: deliberate, minimal public items; breaking changes flagged, semver checks run, changes recorded; no internal types leak through public signatures under any feature combination; features additive, never silently changing behaviour.
14. **docs.rs-ready documentation.**
    Check: every public item documented; `# Errors` on fallible and `# Panics` on panicking functions; runnable `# Examples` where they help; crate- and module-level docs say what it is and how to start; doc examples tested.
15. **Contributor experience** — a stranger can clone and go.
    Check: `cargo build` and `cargo test` work out of the box on default features, no hidden setup; no machine paths, internal tracker ids or private references in shipped files; error messages say what went wrong and what to do; a README per crate (purpose, install, minimal example); CONTRIBUTING and CHANGELOG current.
16. **Dependency and supply-chain hygiene.**
    Check: minimal, well-adopted dependencies (the crate cargo/uv/ruff use, with adoption evidence); cargo-deny licences and advisories pass; no unused dependencies (cargo-machete); optional dependencies behind features; no secrets (gitleaks).
17. **Portable and reproducible.**
    Check: no platform assumptions without `cfg` gating; deterministic output (no hash-map ordering leaking into results); toolchain and tool versions pinned in the repo; tests don't depend on the local machine, the network or timing.
18. **Use what everyone uses; don't roll your own.**
    Check: an existing standard tool or well-adopted crate is used wherever one fits (the crate cargo/uv/ruff use, with adoption evidence); no home-grown parser, runner, format, harness or checker where a standard one exists; tiny glue is inlined, not built into a framework.
19. **Don't write a lot of code.**
    Check: the change is the smallest that meets the goal; no speculative generality, no new abstraction without two real users, no scaffolding or frameworks; net lines justified; deleting beats adding.
20. **No leaks; purposeful placement.**
    Check: every file, type and function lives where its concept's placement rules put it (Tastemaker placements, hex layer, concept dir); no stray or catch-all placement; nothing visible beyond what its consumers need; no test seam, fixture or internal type reachable from production or the public API.
21. **One of N looks for the shared concern.**
    Check: when changing one of N siblings (a language provider, a linter adapter, a benchmark profile), ask whether the change is really common to all N. If it is, it belongs in the shared language contract crate or the harness, once, not copied into one sibling or repeated in each. A provider-specific change states why the others don't need it.
