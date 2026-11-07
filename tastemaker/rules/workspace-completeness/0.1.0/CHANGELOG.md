# workspace-completeness 0.1.0

The v2 release of `arch.workspace-completeness` 0.1.0, under its unprefixed name.

## Changed

- `rule.toml` is format `version = 3`, and its scenarios are in the file.
- The `workspaces` and `packages` params are gone. The rule reads its scope: every member
 package of a Cargo workspace whose root manifest is in the entry's scope is in scope too.
 A finding's subject is the workspace root `Cargo.toml`, one per missing member.

## Moving up from arch.workspace-completeness 0.1.0

- Name the rule `workspace-completeness` at version `0.1.0` instead of
 `arch.workspace-completeness`, placed by the shared rule block with `include = ["**"]`.
- Drop `workspaces` and `packages`. A member the entry excludes is a finding: bring it into
 scope or remove it from the Cargo workspace.

## Fixtures

Converted to the v2 fixture format. The v1 `packages` selection becomes the entry's scope:
`include = ["**"]`, with an `exclude` for each member v1 left unselected.

- `unselected-member` and `nested-member-unselected` exclude the unselected member; the
 finding's subject is now `Cargo.toml` (v1 named the member).
- `empty-selection` places only the root manifest (`include = ["Cargo.toml"]`): one finding
 on `Cargo.toml` per member, as v1 had one per member.
- `excluded` and `nested-workspace` exclude `crates/support`, which is not a member of the
 root workspace: pass.
- Dropped `unavailable-metadata`: its verdict was `unknown`, which the v2 fixture format
 does not express.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.
