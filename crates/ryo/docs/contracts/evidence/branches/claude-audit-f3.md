# Leak re-check: claude/audit-f3 at 1b5d83ba0b

Source 1b5d83ba0b6a100a74b5c6a2df8c9188f834302c, delta from a6fd8d8254 (the earlier FAIL). The delta changes 3 files (+9/-5): `sdk/src/resources/languages/contract.rs`, `execution_store/src/application/file_dag/relations.rs` and `execution_store/src/application/file_dag/workspace/test.rs`.

## 1 Proof where the contract lives: PASS
Nothing changed under benchmarks/ or languages/. The inspection count is still proved in the owning crate (`workspace/test.rs:145,168`).

## 2 Every enforcement claim fires: PASS
The delta adds no enforcement claim. ci/, tastemaker/ and both manifests are identical to a6fd8d8254, so the three sdk_api.py plants of the earlier review still hold (`sdk_api.py:155,172,222`, count 1 each). The test edit is only a rename, and the assertion (delta == 1) can still fail. I planted nothing new because the guard hook refuses cargo builds outside an integration worktree.

## 3 Boundaries hold across crates: PASS (earlier FAIL closed)
- `contract.rs:76-78` re-exports `ParseEngine` and `ProjectEngine` from under the allowed prefix `ryo::resources::languages` (`crates/ryo/Cargo.toml:17-20`).
- `sdk_api.py:101-105` counts those `use` ids as surface, so `Language::parse_engine` and `Language::project` (`types.rs:119-130`) no longer leak.
- The default surface does not change. The `contract` module is `#[cfg(feature = "fixtures")]` (`resources/languages.rs:3-4`), the SDK has no default feature (`sdk/Cargo.toml:40-43`), and semver-checks runs on default features.

I proved this by reading the code, not by running it, because the rustdoc build was refused.

## 4 One implementation per concept: PASS
The re-exports reuse the single trait definitions in ryo_language. `admitted_inspections()` still derives from the existing counters.

## 5 Layer direction: PASS
The re-exports sit in the SDK's inbound resource module, next to `contract::read`. `admitted_inspections` is still `#[cfg(test)] pub(super)`.

## 6 Test seams stay out of production: PASS
Both re-exports are reachable only with `fixtures`, which only dev-dependencies switch on. Nothing else changed.

## Advisories
- `sdk/api.txt` was not regenerated and lists no `contract` items, while the listing is now read with `--all-features`. The sdk-api step fails loudly until the integrator records `cargo +nightly-2026-08-01 public-api -ss -p idiomatic-sh-ryo-sdk --all-features`.
- Carried: the fixtures-feature plants were not re-run, and semver-checks runs on default features only.
- Enforcement candidate `feature-gated-sdk-method-hands-out-an-internal-trait-1090d557` is carried forward as `fixed`.

VERDICT: PASS
