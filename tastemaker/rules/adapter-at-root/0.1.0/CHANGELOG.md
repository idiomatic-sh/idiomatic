# adapter-at-root 0.1.0

The v2 release of `arch.adapter-at-root` 0.2.0, under its unprefixed name.

## Changed

- `rule.toml` is format `version = 3`, and its scenarios are in the file.
- The `paths`, `roots` and `compositions` params are gone. The rule entry scopes the rule;
 the shared rule block places it with `include = ["**"]` and
 `exclude = ["**/composition.rs", "**/main.rs", "**/adapters/**", "**/tests/**", "**/benches/**", "**/*_test.rs"]`.
 Any mention of an `adapters` path (`*::adapters::*` or `adapters::*`) in a file in scope is
 a finding.
- The "crossing" half is gone: v1 also refused an adapter naming another adapter from inside
 an `adapters` directory. The entry excludes `**/adapters/**`, so that check has no input.
- The `adapters.rs` registration module is no longer a root. It declares each
 implementation's module; one that names an adapter's path is a finding.

## Moving up from arch.adapter-at-root 0.2.0

- Name the rule `adapter-at-root` at version `0.1.0` instead of `arch.adapter-at-root`.
- Drop `paths`, `roots` and `compositions`. Place the rule with the shared rule block's
 `include`/`exclude`; a further root is another `exclude` on the entry.
- Move any adapter selection out of `adapters.rs` into `composition.rs`.

## Fixtures

Converted to the v2 fixture format; every fixture uses the shared rule block's `include`
and `exclude`.

- `registration-module` is now a violation: `src/port/adapters.rs` names
 `crate::port::adapters::native::adapter::PortNativeAdapter`, and `adapters.rs` is in scope.
- Dropped `adapter-constructs-adapter`: it tested only the crossing half, and every file it
 flags sits under `adapters/`, which the entry excludes.

## Help that says how to act

Every finding is printed with the rule's help, so the help says what to change and
where, and whether an ignore with a reason is ever legitimate, with the exact ignore
when it is. The rule's text states its reasons itself and cites no design ids.

## The port's name

The help names the port by the suffix the `part` spec gives it, `*Port`, beside a hex crate's `*Interface`. The query is unchanged.

## Fixtures

- A port's `fixtures/` files may name an adapter: the shared rule block leaves out `**/fixtures/**` (fixtures `fixture-names-adapter`, `fixture-lookalike`).
