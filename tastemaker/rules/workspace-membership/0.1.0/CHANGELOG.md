# workspace-membership 0.1.0

The v2 release of `arch.workspace-membership` 0.1.0, under its unprefixed name.

## Changed

- `rule.toml` is format `version = 3`, and its scenarios are in the file.
- The `workspaces` and `packages` params are gone. The rule reads the placed crates
 (`tastemaker.placed`): every `Cargo.toml` a `cargo-package` template entry places in the
 entry's scope is a member of a Cargo workspace or a workspace root manifest itself. A
 finding's subject is that manifest.

## Moving up from arch.workspace-membership 0.1.0

- Name the rule `workspace-membership` at version `0.1.0` instead of
 `arch.workspace-membership`, placed by the shared rule block with `include = ["**"]`.
- Drop `workspaces` and `packages`. A manifest that is deliberately outside every workspace
 either becomes its own workspace root (`[workspace]`) or is excluded from the entry.

## Fixtures

Converted to the v2 fixture format, with `include = ["**"]` unless noted. Each fixture's
`tastemaker.toml` places every crate manifest with `cargo-package` and reads only this
rule's findings.

- `manifest-inside-workspace-workspace-neither-lists-nor` (v1's `unlisted`) fails at
 `crates/support/Cargo.toml`: the Rust provider reads a manifest Cargo neither lists nor
 excludes as in no workspace, so the rule reports it, not `#unknown`.

- `nested-workspace` is now a pass: `crates/support/Cargo.toml` declares `[workspace]`, so
 it is a workspace root manifest. v1 failed it because the selection named it as a package
 of the outer workspace; v2 has no selection.
- `empty-selection` now places only the root manifest (`include = ["Cargo.toml"]`), which
 is a workspace root: pass.
- `extra-unselected` excludes `crates/extra`: a member outside the scope passes.
- Dropped `narrowed-members` and `unavailable-metadata`: their verdict was `unknown` (Cargo
 metadata fails on the tree), which the v2 fixture format does not express.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.

Fixture `dependency-outside` (pass): a member's path dependency on a package outside the run leaves the members read, so no member is reported (where Tastemaker's own check reported `cli` and `crates/core`, which depend on ryo's SDK).
