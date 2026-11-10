# Leak review: claude/audit-f3 at 3b08375797

Base `85ca33800e`. The delta is one commit, which closes the 9 SDK API reaches the sdk-api step reported at the base (`the checkpoint validation directory's sdk-api-red-at-85ca33800e.log`).

## 1 Proof where the contract lives: PASS
The one benchmark edit, `benchmark/src/outbound/engine/adapters/local/schemas_test.rs:48`, still asserts the benchmark worker's own module text. It now reads it through `RyoQlQuery`'s public `AsRef<str>` (`sdk/src/domain/query/types.rs:125-129`, which returns the same `root_text`), so the assertion is just as strong. The four permission variants stay proven in the SDK at `lint/fixtures/permission.rs:60-69`.

## 2 Every enforcement claim fires: PASS
- **The real-tree violation:** I ran the head's `leaks()` (`sdk_api.py:72-132`) over the base tree's own rustdoc JSON (`the integration tree's target/doc/ryo.json`, written by the red run). It reports **8**. That is the base's 9 minus `From<Language> for LanguageConnection`, which the corrected `ALLOWED_IMPLS` (`sdk_api.py:35`) now admits. The base script reported all 9.
- **The unit tests:** `sdk_api_test.py` passes 16/16. Tests `:120` and `:129` still catch an impl for an unlisted internal type, and an internal type that the listed impl names.
- **No path turns a failure into a pass:** the edit changes only the allowlist string, which is matched by exact tuple membership (`:65`).
- **No plant at the head:** the guard hook refuses `cargo rustdoc` outside an integration worktree. The head does compile in a `git archive` copy: the SDK on default features, the SDK with `--all-features --tests`, and the benchmark with `--all-features --tests`.

## 3 Boundaries hold across crates: PASS
All 9 reaches are closed, checked by reading:
- `CaseSource.executable` and the `ryo::linter` re-export now name the SDK's own enum (`lint/types.rs:248`, `linter.rs:30`).
- `QueryModuleSource::id` and `text` are now `#[cfg(test)] pub(crate)`, and `RyoQlQuery::root_text` is `pub(crate)` (`query/types.rs:45-55`, `:112-116`).
- The three `From` impls are now `pub(crate)` `into_engine` methods (`observation/types.rs:69-99`).

The base JSON has five impls for a foreign type. `From<Language> for LanguageConnection` (`composition.rs:254`) is the only one left for `LanguageConnection`. The `CellKind` impl passes because `languages.rs:78` re-exports the type from the allowed module.

Nothing outside the SDK relied on a sealed item:
- `QueryModuleSource` callers use only `try_new` (benchmark `schemas.rs:182,205`, `tool/schemas.rs:83`, `writer.rs:30`).
- `root_text`, `id` and `text` are called only in the SDK's own tests (`schemas_test.rs:238,248,255`).
- Every `ExecutablePermission` user outside the SDK names `ryo::linter::ExecutablePermission` (benchmark, Tastemaker core `schemas.rs:8,536`, the ruff `engine_test` files) or uses `Default::default()` (cli replay). None names the engine type through the SDK.

No doc, example or Tastemaker sample names a sealed item.

## 4 One implementation and one representation per concept: PASS
The SDK enum (`sdk/src/domain/file/types.rs:31-43`) and the engine enum (`values/src/domain/files/types.rs:9-20`) match exactly:
- the same variants in the same order, so the derived `Ord` agrees;
- the same `#[default] Unknown`;
- the same derives.

`into_engine` is an exhaustive one-to-one match with no wildcard, and there is no reverse conversion. The SDK enum is the SDK's one edge copy in `src/domain/`, which `tastemaker/rules/one-type-per-name/0.2.0/CHANGELOG.md:13-15` allows by design. The observation conversions are still one function each.

## 5 Layer direction: PASS
The SDK's domain types convert inward to `ryo_values`, and the resources call those conversions (`composition.rs:229`, `lint/schemas.rs:676-680`, `validate/schemas.rs:297-301`). No inbound item crosses a crate.

## 6 Test seams stay out of production: PASS
The only test-scoped items are the `#[cfg(test)]` accessors `id` and `text` (`query/types.rs:45-55`), and only `schemas_test.rs` calls them. The edit to `fixtures/permission.rs:18` only changes an import path.

## Advisories
- **`api.txt` is stale (blocks the landing, not a leak):** `crates/ryo/crates/sdk/api.txt` still records every removed item: `:468`, `:637`, `:1273`, `:1650`, `:2544`, `:2660`, `:2736`, `:2809-2810` and `:2955`. It also lacks the contract-module items flagged earlier. The leak step now passes, but `sdk_api.py:206-209` fails the listing comparison. Record the listing with `cargo +nightly-2026-08-01 public-api -ss -p idiomatic-sh-ryo-sdk --all-features` before landing.
- **The allowlist is broader than one impl:** `allowed_impl` (`sdk_api.py:58-65`) checks only the impl's target type, so any trait impl for `LanguageConnection` with SDK-local arguments would pass. This predates the edit, and the real tree has exactly one such impl. Enforcement candidate `allowed-sdk-impl-keyed-by-its-target-type-alone-641c27f1`: key the allowlist on the trait, its arguments and the target type.
- **Semver record (unreleased, default surface, against `85ca33800e`):**
  - removed: the methods `RyoQlQuery::root_text`, `QueryModuleSource::id` and `QueryModuleSource::text`;
  - removed: the three `From` impls into `ryo_values`/`StepScope`;
  - changed: `ryo::linter::ExecutablePermission` is now an SDK-owned enum instead of a re-export of the engine type, with the same variants and default.

  None of these exist at `baseline-rev` `e48028e553`, so `semver-checks` stays quiet.
- **Where sdk-api should run:** it is the only tool in group `api` (`asymptote.toml:41-46`). The checkpoint runs `--groups lint,check`, and `commands.json` has no `sdk_api` entry, so it never ran at a checkpoint. That is how the 9 reaches landed. Run it at every checkpoint, either as a `commands.json` entry in `run.py` with its log under evidence, or by adding `api` to the checkpoint's `--groups`.
- **Carried from the earlier review:** `semver-checks` runs on default features only.
- **Carried candidate:** `feature-gated-sdk-method-hands-out-an-internal-trait-1090d557`, `instance_status` fixed.

VERDICT: PASS
