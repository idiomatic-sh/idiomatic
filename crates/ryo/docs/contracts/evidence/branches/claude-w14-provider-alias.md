# Leak review: claude/w14-provider-alias (re-review)

Source `6e7265eaac8b60a6ca4a0758ea37667f38456364`, base `a19f3047c624f2e2c03d0cf85b12ef527aa13f9d`. The previous review, of `a33a4c13f1`, was FAIL: 2.

## Check 2 Every enforcement claim fires: PASS
- **(A) Shadowing.** `through_uses` filters the in-scope bindings and takes the one with the smallest scope. A file-top `use` (no scope) ranks last, so the innermost `use` wins, as in Rust. The new shadow case expects the `other.rs` target; reverting to `find` would give `shared.rs` and fail it.
- **(B) Module identity.** `scope_of` records the span of the innermost block or inline module around every `use` that is not at the file top, and the span must hold the path. Same-named inline modules in sibling functions therefore stay apart; the new case would fail on the name chain alone.
- **Spans.** I recomputed every span in the six `aliases_test` cases and in `use_scopes` (`a:7..45 b:16..26 c:33..43`) from the source text; all match.
- **(C) Unreadable notes.** Each note is decoded once. Empty bytes mean no note; any other bytes that fail to decode propagate `NotesError`, which becomes `ProjectError::Context`. A new case expects the refusal.
- **Build.** `cargo check -p idiomatic-sh-ryo-rust --all-targets` on an archive copy gives 0 errors. Its one warning (`contract.rs:311`) predates this branch.
- **Not run yet.** The tests and the after-plant have not run on this branch. They are listed for the 333 integrator below.

## The builder's tm claim: verified
tm's `in_scope` (`f1c9e1c968`, `import_rows/service.rs:144-169`) never compares module names:
- it takes the innermost scope span holding the `use`, from `syntax-tree` `stmt-list`/`item-list` rows;
- it requires that span to hold the edge;
- it shuts on any `item-list` span inside it that holds the edge.

Same-named modules have different spans, so variant B does not apply to tm.

## Advisory: the refusal does not name the file
The message `the bytes are not exactly one note` does not say which file. It is a loud refusal, not a silent loss, so it is not a leak. The no-string-literals rule does not force the omission: a constant prefix plus the file's path (`request.files` at the same index), or a `NotesError` variant carrying the path, would name the file without a literal.

## Advisory: two files at exactly 400 lines
`links/service.rs` and `project/contract.rs` are each exactly 400 lines. `file-length` 0.3.0 refuses only above 400, so both pass, but neither has headroom: the next edit to either must split it by role.

## What the 333 integrator must run (updated)
1. `cargo test -p idiomatic-sh-ryo-rust --lib -- links::service_test fact_extraction::service_test inbound::contracts::project outbound::syntax::adapters::ra_ap_syntax::adapter_test outbound::notes::adapters::postcard`
2. **The after-plant**, on `scratchpad/w14-alias-plant` with the 333 build's ryo and a fresh store:

   ```
   ryo query run --text 'let rows = select * from "source-dependency"; emit rows subject "source" key("target", "span");'
   ```

   `fn b`'s `ryo::query::ValidateRequest` must target the ryo crate's `query.rs`, not the aliased module, with no `ProjectError`.
3. **tm on the real-tree sibling plant**, using a tm built on this provider: one `provider-imports` finding at `fn b`'s line.
4. **One whole-repository Rust run on a fresh store**, for example tm check of `crates/ryo`: there must be no `ProjectError::Context`. The links now refuse instead of skipping, so this confirms no real file's note bytes are non-empty and undecodable.

## Checks 1, 3, 4, 5, 6: PASS
- **1:** the proofs live in the provider.
- **3:** the `From` into the SDK's public error type is implemented where `NotesError` is local; no boundary changes.
- **4:** one alias resolver, and one decode per note.
- **5:** layer direction unchanged.
- **6:** the fixtures are test-only.

## Enforcement candidates
All four are fixed:
- `alias-binding-not-nearest-784e1025`
- `module-identity-by-name-chain-23c55763`
- `note-decode-failure-is-absence-cb263667`
- `provider-alias-resolved-across-blocks-3c609ce6`

VERDICT: PASS
